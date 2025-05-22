use std::{env, fs, path::Path};
use typify::{TypeSpace, TypeSpacePatch, TypeSpaceSettings};

fn main() {
    schema_to_rust(
        "vendor/schema_catalog_schema.json",
        "generated",
        "schema_catalog_schema.rs",
    );
}

fn schema_to_rust(schema_file: &str, out_dir: &str, out_file: &str) {
    let content = std::fs::read_to_string(schema_file).unwrap();
    let mut schema = serde_json::from_str::<schemars::schema::RootSchema>(&content).unwrap();

    // Add a title so that typify knows what to name the struct.
    {
        match &schema.schema.metadata {
            Some(_metadata) => {
                panic!("Schema metadata is not missing. Apparently the schema changed. Update build.rs to handle the new schema.");
            }
            None {} => {}
        };
        schema.schema.metadata = Some(Box::new(schemars::schema::Metadata {
            title: Some("Catalog".to_string()),
            ..Default::default()
        }));
    }

    let mut type_space = TypeSpace::new(
        TypeSpaceSettings::default()
            .with_derive("Debug".to_string())
            .with_derive("PartialEq".to_string())
            .with_derive("Eq".to_string())
            .with_derive("Clone".to_string())
            .with_patch(
                "CatalogGroupsItem".to_string(),
                TypeSpacePatch::default().with_rename("CatalogGroup".to_string()),
            )
            .with_patch(
                "CatalogGroupsItemSchemasItem".to_string(),
                TypeSpacePatch::default().with_rename("Schema".to_string()),
            )
            .with_map_type("std::collections::BTreeMap".to_string()),
    );
    // type_space.add_ref_types(schema.definitions).unwrap();
    type_space.add_root_schema(schema).unwrap();

    let contents =
        prettyplease::unparse(&syn::parse2::<syn::File>(type_space.to_stream()).unwrap());

    let mut out_path = Path::new(&env::var("OUT_DIR").unwrap()).to_path_buf();
    out_path.push(out_dir);
    fs::create_dir_all(out_path.clone()).unwrap();
    out_path.push(out_file);
    fs::write(out_path, contents).unwrap();
}
