use anyhow::{Context as _, Ok, Result};
use clap::{arg, Parser, Subcommand};
use json_schema_catalog_rs::{catalog_from_groups, group_from_schema, Catalog, Index};

#[derive(Parser)]
#[command(
    name = "json-schema-catalog",
    about = "A tool for working with JSON Schema Catalogs",
    version,
    author
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Check a JSON schema catalog file for validity
    Check(CheckCommand),
    /// Look up a schema location by its id in a JSON schema catalog file
    Lookup(LookupCommand),
    /// Replace "$ref", "$schema" occurrences in a JSON file with the corresponding physical file location
    Replace(ReplaceCommand),
    /// Create a new JSON Schema Catalog file from a set of JSON schema files
    New(NewCommand),
}

#[derive(Parser)]
struct CheckCommand {
    /// Path to the catalog file
    #[arg(
        help = "Path to the JSON schema catalog file. This checks the individual files for being valid JSON, but NOT for being a valid schema."
    )]
    catalog_file: String,

    #[arg(
        help = "Disable checking for matching schema ids.",
        long("no-check-schema-id"),
        default_value = "true"
    )]
    require_matching_id: bool,
}
impl CheckCommand {
    fn run(&self) -> Result<()> {
        let opts = json_schema_catalog_rs::CheckOptions {
            require_matching_id: true,
        };
        let catalog: Catalog = serde_json::from_str(&std::fs::read_to_string(&self.catalog_file)?)?;
        catalog.check(&opts, &self.catalog_file)?;
        Ok(())
    }
}

struct Context {
    catalogs: Vec<(String, Catalog)>,
    index: Index,
}
impl Context {
    fn new(extra_files: &Vec<String>, verbose: bool) -> Result<Self> {
        let xdg_catalogs = xdg::BaseDirectories::new()
            .list_data_files_once("json-schema-catalogs")
            .iter()
            .flat_map(|path| {
                Some(path).into_iter().flat_map(move |entry| {
                    if entry.extension().map_or(false, |ext| ext == "json") {
                        Some(entry.to_str().unwrap().to_string())
                    } else {
                        None
                    }
                })
            })
            .collect::<Vec<String>>();

        let mut catalog_files = xdg_catalogs;
        catalog_files.extend(extra_files.iter().cloned());

        let catalogs = catalog_files
            .iter()
            .map(|file| {
                if verbose {
                    eprintln!("Parsing catalog {:?}", file);
                }
                let c: Catalog = serde_json::from_str(&std::fs::read_to_string(&file)?)?;
                // Get the directory of the file
                let dir = std::path::Path::new(file)
                    .parent()
                    .with_context(|| format!("Could not get parent directory of {}", file))?;
                // Convert to string
                let dir = dir.to_str().with_context(|| {
                    format!("Could not convert parent directory of {} to string", file)
                })?;

                Ok((dir.to_string(), c))
            })
            .collect::<Result<Vec<_>, _>>()?;

        let mut r = Self {
            catalogs,
            index: Index::new(),
        };
        for (dir, catalog) in &r.catalogs {
            catalog.index(dir, &mut r.index);
        }
        Ok(r)
    }
}

#[derive(Parser)]
struct LookupCommand {
    /// The schema id to look up
    #[arg(help = "The schema id to look up")]
    schema_id: String,

    /// Path to the catalog files
    #[arg(
        help = "Path to the JSON schema catalog file. This checks the individual files for being valid JSON, but NOT for being a valid schema. Files can also be passed via XDG_DATA_HOME and XDG_DATA_DIRS, as per ${dirs}/json-schema-catalogs/*.json"
    )]
    catalog_files: Vec<String>,

    /// Verbose output
    #[arg(help = "Enable verbose output", long, default_value = "false")]
    verbose: bool,
}
impl LookupCommand {
    fn run(&self) -> Result<()> {
        let context = Context::new(&self.catalog_files, self.verbose)?;

        let schema = context
            .index
            .get_path(self.schema_id.as_str())
            .ok_or_else(|| {
                anyhow::anyhow!("Schema with id {} not found in catalog", self.schema_id)
            })?;
        println!("{}", &schema);

        Ok(())
    }
}

#[derive(Parser)]
struct ReplaceCommand {
    /// Path to the catalog file
    #[arg(
        help = "Path to JSON file(s) in which to replace id occurrences with the corresponding physical file location."
    )]
    json_files: Vec<String>,

    /// Extra catalog files
    #[arg(
        help = "Extra catalog files to use. These are in addition to the ones found in XDG_DATA_HOME and XDG_DATA_DIRS.",
        long = "extra-catalogs"
    )]
    extra_catalogs: Vec<String>,

    /// Verbose output
    #[arg(help = "Enable verbose output", long, default_value = "false")]
    verbose: bool,

    /// Ignore URIs for which we cannot find a schema
    #[arg(
        help = "Ignore URIs for which we cannot find a schema",
        long = "ignore-unknown",
        default_value = "false"
    )]
    ignore_unknown: bool,
}
impl ReplaceCommand {
    fn replace(&self, context: &Context, value: &mut serde_json::Value) -> Result<()> {
        let mut status = Ok(());
        match value {
            serde_json::Value::Object(map) => {
                for (key, value) in map.iter_mut() {
                    if key == "$ref" || key == "$schema" {
                        match context.index.get_path(value.as_str().unwrap()) {
                            Some(path) => {
                                if self.verbose {
                                    eprintln!(
                                        "Replacing {} field, old: {} new: {}",
                                        key, value, path
                                    );
                                }
                                *value = serde_json::Value::String(path);
                            }
                            None {} => {
                                if !self.ignore_unknown {
                                    status = Err(anyhow::format_err!(
                                        "Could not find schema with id {}",
                                        value.as_str().unwrap()
                                    ))
                                } else if self.verbose {
                                    eprintln!(
                                        "Ignoring unknown schema id {}",
                                        value.as_str().unwrap()
                                    );
                                }
                            }
                        };
                    } else {
                        self.replace(context, value)?;
                    }
                }
            }
            serde_json::Value::Array(array) => {
                for value in array.iter_mut() {
                    self.replace(context, value)?;
                }
            }
            _ => {}
        }
        status
    }
    fn run(&self) -> Result<()> {
        let context = Context::new(&self.extra_catalogs, self.verbose)?;

        for file in &self.json_files {
            let content = std::fs::read_to_string(file)?;
            let mut value: serde_json::Value = serde_json::from_str(&content)?;
            self.replace(&context, &mut value)?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }

        Ok(())
    }
}

#[derive(Parser)]
struct NewCommand {
    /// Path to the catalog file
    #[arg(help = "Paths to JSON Schema files")]
    schema_files: Vec<String>,

    /// Set the name of the schema group not to the individual schema file titles,
    /// so that they can be grouped together.
    #[arg(
        long = "group-name"
    )]
    group_name: Option<String>,

    /// Set the catalog name
    #[arg(
        long = "catalog-name",
        default_value = "Catalog"
    )]
    catalog_name: String,
}
impl NewCommand {
    fn run(&self) -> Result<()> {
        let mut groups = Vec::new();
        for file in &self.schema_files {
            let content = std::fs::read_to_string(file)?;
            let value: serde_json::Value = serde_json::from_str(&content)?;
            let mut group = group_from_schema(file, &value)?;
            if let Some(name) = &self.group_name {
                group.name = name.clone();
            }
            groups.push(group);
        }
        let catalog = catalog_from_groups(self.catalog_name.clone(), groups)?;
        println!(
            "{}",
            serde_json::to_string_pretty(&catalog).context("Failed to serialize catalog")?
        );
        Ok(())
    }
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Check(cmd) => cmd.run(),
        Commands::Lookup(cmd) => cmd.run(),
        Commands::Replace(cmd) => cmd.run(),
        Commands::New(cmd) => cmd.run(),
    }
    .unwrap_or_else(|e| {
        // ANSI bold red
        eprintln!("\x1b[1;31merror:\x1b[0m");
        for cause in e.chain() {
            eprintln!("  {}", cause);
        }
        std::process::exit(1);
    });
}
