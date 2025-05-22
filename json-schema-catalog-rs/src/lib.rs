use anyhow::{bail, Context as _, Ok, Result};
use schemars::schema::RootSchema;
use std::path::Path;

pub struct CheckOptions {
    pub require_matching_id: bool,
}
impl CheckOptions {
    pub fn new() -> Self {
        CheckOptions {
            require_matching_id: true,
        }
    }
}
impl Default for CheckOptions {
    fn default() -> Self {
        Self::new()
    }
}

include!(concat!(
    env!("OUT_DIR"),
    "/generated/schema_catalog_schema.rs"
));

// struct declared in generated file
impl Catalog {
    pub fn check(&self, opts: &CheckOptions, file_name: &str) -> Result<()> {
        let base_dir = std::path::Path::new(file_name).parent().unwrap();
        for group in &self.groups {
            group
                .check(opts, base_dir)
                .with_context(|| format!("in catalog {}, file {}", self.name, file_name))?;
        }
        Ok(())
    }
    pub fn index(&self, basedir: &str, index: &mut Index) {
        for group in &self.groups {
            group.index(basedir, index);
        }
    }
}
// struct declared in generated file
impl CatalogGroup {
    pub fn check(&self, opts: &CheckOptions, base_dir: &Path) -> Result<()> {
        if self.name.is_empty() {
            bail!("Group name is empty");
        }
        if self.base_location.is_empty() {
            bail!("Group base location is empty".to_string());
        }
        let base_dir = base_dir.join(&self.base_location);
        for schema in &self.schemas {
            schema
                .check(opts, &base_dir)
                .with_context(|| format!("in catalog group {}", self.name))?;
        }
        Ok(())
    }
    pub fn index<'a>(&self, basedir: &'a str, index: &mut Index) {
        for schema in &self.schemas {
            schema.index(basedir, &self.base_location, index);
        }
    }
}
// struct declared in generated file
impl Schema {
    pub fn check(&self, opts: &CheckOptions, base_dir: &Path) -> Result<()> {
        if self.id.is_empty() {
            bail!("Schema id is empty".to_string());
        }
        if self.location.is_empty() {
            bail!("Schema location is empty".to_string());
        }
        let location = base_dir.join(&self.location);
        let location = location.as_path();
        if let Err(e) = std::fs::metadata(&location) {
            bail!(
                "Could not access {} as file: {}",
                location.to_string_lossy(),
                e
            );
        }
        let content = match std::fs::read_to_string(&location) {
            Err(e) => {
                bail!(
                    "Could not read {} as file: {}",
                    location.to_string_lossy(),
                    e
                );
            }
            Result::Ok(content) => content,
        };
        let value = match serde_json::from_str::<serde_json::Value>(&content) {
            Result::Ok(value) => value,
            Result::Err(e) => {
                bail!(
                    "Could not parse {} as JSON: {}",
                    &location.to_string_lossy(),
                    e
                );
            }
        };

        // If an id is present, it must match the recorded schema id
        if opts.require_matching_id {
            if let Some(id) = value.get("id") {
                if id != &self.id {
                    bail!(
                        "Recorded schema id {} does not match id {} in file {}",
                        self.id,
                        id,
                        location.to_string_lossy()
                    );
                }
            }
        }

        // Idea: validate the whole schema, optionally

        Ok(())
    }

    fn index(&self, basedir: &str, base_location: &str, index: &mut Index) {
        index.by_id.insert(
            self.id.clone(),
            IndexEntry {
                basedir: basedir.to_string(),
                base_location: base_location.to_string(),
                file: self.location.clone(),
            },
        );
    }
}

struct IndexEntry {
    basedir: String,
    base_location: String,
    file: String,
}
impl IndexEntry {
    fn get_path(&self) -> String {
        let mut path = std::path::PathBuf::new();
        path.push(&self.basedir);
        path.push(&self.base_location);
        path.push(&self.file);
        return path.to_string_lossy().to_string();
    }
}

/// An index for looking up schema files by their id.
///
/// The index is filled by calling the `index` method on a `Catalog`, `CatalogGroup` or `Schema`.
pub struct Index {
    by_id: std::collections::HashMap<String, IndexEntry>,
}
impl Index {
    pub fn new() -> Self {
        Index {
            by_id: std::collections::HashMap::new(),
        }
    }
    fn get_entry(&self, id: &str) -> Option<&IndexEntry> {
        self.by_id.get(id)
    }
    pub fn get_path(&self, id: &str) -> Option<String> {
        let entry = self.get_entry(id)?;
        Some(entry.get_path())
    }
}

/// Generate a singleton group from a schema file.
pub fn group_from_schema(file: &str, schema: &serde_json::Value) -> Result<CatalogGroup> {
    let schema = serde_json::from_value::<RootSchema>(schema.clone())?;
    let id = schema
        .schema
        .metadata
        .as_ref()
        .and_then(|m| m.id.clone())
        .ok_or_else(|| anyhow::format_err!("Schema {} does not have an $id field", file))?;
    let base_dir = std::path::Path::new(file)
        .parent()
        .ok_or_else(|| anyhow::format_err!("Could not get parent directory of {}", file))?;
    let file_name = std::path::Path::new(file)
        .file_name()
        .ok_or_else(|| anyhow::format_err!("Could not get file name from {}", file))?;
    let name = schema
        .schema
        .metadata
        .as_ref()
        .and_then(|m| m.title.clone())
        .ok_or_else(|| anyhow::format_err!("Schema {} does not have a title field", file))?
        .clone();
    Ok(CatalogGroup {
        name,
        base_location: base_dir.to_string_lossy().to_string(),
        schemas: vec![Schema {
            id,
            location: file_name.to_string_lossy().to_string(),
        }],
    })
}

fn group_key(group: &CatalogGroup) -> (String, String) {
    (group.base_location.clone(), group.name.clone())
}

/// Merge groups into a single catalog. Groups with matching base_location and name
/// are merged into a single group.
pub fn catalog_from_groups(name: String, groups: Vec<CatalogGroup>) -> Result<Catalog> {
    let mut groups = groups.clone();
    groups.sort_by_key(group_key);
    let groups = groups
        .chunk_by(|a, b| group_key(a) == group_key(b))
        .map(|chunk| {
            let group = chunk[0].clone();
            let mut schemas = vec![];
            for g in chunk {
                schemas.extend(g.schemas.clone());
            }
            schemas.sort_by_key(|s| s.id.clone());
            Ok(CatalogGroup {
                name: group.name,
                base_location: group.base_location,
                schemas,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Catalog { name, groups })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_schema() {
        let catalog_value = json!({
            "name": "foo",
            "groups": [
              {
                "name": "I think we'll mostly ignore names",
                "baseLocation": "vendor",
                "schemas": [
                  {
                    "id": "foo",
                    "location": "schema_catalog_schema.json",
                  }
                ]
              }
            ]
        });
        let catalog_expect = Catalog {
            name: "foo".to_string(),
            groups: vec![CatalogGroup {
                name: "I think we'll mostly ignore names".to_string(),
                base_location: "vendor".to_string(),
                schemas: vec![Schema {
                    id: "foo".to_string(),
                    location: "schema_catalog_schema.json".to_string(),
                }],
            }],
        };
        // parse the catalog json with serde
        let catalog: Catalog = serde_json::from_value(catalog_value).unwrap();
        assert_eq!(catalog, catalog_expect);
    }

    #[test]
    fn example_check() {
        let catalog: Catalog =
            serde_json::from_str(&std::fs::read_to_string("./example.json").unwrap()).unwrap();
        catalog
            .check(&Default::default(), "./example.json")
            .unwrap();
    }

    #[test]
    fn relative_check() {
        let catalog: Catalog =
            serde_json::from_str(&std::fs::read_to_string("test/example.json").unwrap()).unwrap();
        catalog
            .check(&Default::default(), "test/example.json")
            .unwrap();
    }

    #[test]
    fn test_lookup() {
        let catalog: Catalog =
            serde_json::from_str(&std::fs::read_to_string("test/example.json").unwrap()).unwrap();
        let mut index = Index::new();
        catalog.index("test", &mut index);
        assert_eq!(
            index.get_path("https://schema.example.com/schema/schema_catalog_schema.json"),
            Some("test/../vendor/schema_catalog_schema.json".to_string())
        );
    }

    #[test]
    fn grouping() {
        let a = group_from_schema(
            "test/example.json",
            &json!({
                "$schema": "http://json-schema.org/draft-07/schema#",
                "$id": "https://schema.example.com/schema/A.json",
                "title": "Catalog",
            }),
        );
        let b = group_from_schema(
            "test/example.json",
            &json!({
                "$schema": "http://json-schema.org/draft-07/schema#",
                "$id": "https://schema.example.com/schema/B.json",
                "title": "Catalog",
            }),
        );
        let c_a = group_from_schema(
            "test/c/a.json",
            &json!({
                "$schema": "http://json-schema.org/draft-07/schema#",
                "$id": "https://schema.example.com/schema/C/A.json",
                "title": "Catalog C",
            }),
        );
        let c_b = group_from_schema(
            "test/cb.json",
            &json!({
                "$schema": "http://json-schema.org/draft-07/schema#",
                "$id": "https://schema.example.com/schema/C/B.json",
                "title": "Catalog Cee",
            }),
        );
        let d = group_from_schema(
            "test/dee/example.json",
            &json!({
                "$schema": "http://json-schema.org/draft-07/schema#",
                "$id": "https://schema.example.com/schema/D.json",
                "title": "Catalog",
            }),
        );

        let groups = vec![
            a.unwrap(),
            b.unwrap(),
            c_a.unwrap(),
            c_b.unwrap(),
            d.unwrap(),
        ];
        let catalog = catalog_from_groups("foo".to_string(), groups).unwrap();
        assert_eq!(
            catalog,
            Catalog {
                name: "foo".to_string(),
                groups: vec![
                    CatalogGroup {
                        base_location: "test".to_string(),
                        name: "Catalog".to_string(),
                        schemas: vec![
                            Schema {
                                id: "https://schema.example.com/schema/A.json".to_string(),
                                location: "example.json".to_string()
                            },
                            Schema {
                                id: "https://schema.example.com/schema/B.json".to_string(),
                                location: "example.json".to_string()
                            }
                        ]
                    },
                    CatalogGroup {
                        base_location: "test".to_string(),
                        name: "Catalog Cee".to_string(),
                        schemas: vec![Schema {
                            id: "https://schema.example.com/schema/C/B.json".to_string(),
                            location: "cb.json".to_string()
                        }]
                    },
                    CatalogGroup {
                        base_location: "test/c".to_string(),
                        name: "Catalog C".to_string(),
                        schemas: vec![Schema {
                            id: "https://schema.example.com/schema/C/A.json".to_string(),
                            location: "a.json".to_string()
                        }]
                    },
                    CatalogGroup {
                        base_location: "test/dee".to_string(),
                        name: "Catalog".to_string(),
                        schemas: vec![Schema {
                            id: "https://schema.example.com/schema/D.json".to_string(),
                            location: "example.json".to_string()
                        }]
                    }
                ],
            }
        )
    }
}
