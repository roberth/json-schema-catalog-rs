[![Hercules CI](https://hercules-ci.com/api/v1/site/github/account/roberth/project/json-schema-catalog-rs/badge)](https://hercules-ci.com/github/roberth/json-schema-catalog-rs/status)

`json-schema-catalog-rs` provides Rust-based tooling around the de facto JSON Schema Catalog standard

# JSON Schema Catalog

A JSON Schema Catalog file provides a mapping from schema URIs to schema locations.
By constructing and using a catalog, you can avoid the need to download and parse schemas from the internet.
This is particularly useful when working with large schemas or when you need to work, test or build offline.

> A json version of [XML Catalogs](https://www.oasis-open.org/committees/entity/spec-2001-08-06.html) for JSON Schemas.

&mdash; [cp-framework-libraries](https://github.com/hmcts/cp-framework-libraries/tree/main?tab=readme-ov-file#json-schema-catalog)

JSON Schema Catalog is not part of any standardization process, as far as I know, as of writing.

# Origin

JSON Schema Catalog was published in the [cp-framework-libraries](https://github.com/hmcts/cp-framework-libraries/tree/main?tab=readme-ov-file#json-schema-catalog) project (by the UK Govt, but this project is neither affiliated nor endorsed by them).
Tea was consumed during this project's creation, fwiw.

# CLI

This crate provides a command line interface (CLI) for working with JSON Schema Catalogs.

```
Commands:
  check    Check a JSON schema catalog file for validity
  lookup   Look up a schema location by its id in a JSON schema catalog file
  replace  Replace "$ref", "$schema" occurrences in a JSON file with the corresponding physical file location
```

Example usage:

```console
$ json-schema-catalog check json-schema-catalog-rs/example.json

$ export XDG_DATA_DIRS=$PWD/json-schema-catalog-rs/test/xdg${XDG_DATA_DIRS:+:$XDG_DATA_DIRS}

$ json-schema-catalog lookup https://schema.example.com/schema/schema_catalog_schema.json
/home/user/src/json-schema-catalog-rs-main/json-schema-catalog-rs/test/xdg/json-schema-catalogs/vendor/schema_catalog_schema.json

# Use `replace` on a catalog with a $schema property
$ json-schema-catalog replace json-schema-catalog-rs/test/example-with-schema.json
{
  "$schema": "/home/user/src/json-schema-catalog-rs-main/json-schema-catalog-rs/test/xdg/json-schema-catalogs/vendor/schema_catalog_schema.json",
  "groups": [
    {
      "baseLocation": "../vendor",
      "name": "json-schema-catalog-rs",
      "schemas": [
        {
          "id": "https://schema.example.com/schema/schema_catalog_schema.json",
          "location": "schema_catalog_schema.json"
        }
      ]
    }
  ],
  "name": "Example Catalog"
}
```

# `replace` is a stop-gap

I recommend that JSON Schema Catalog support be _built in_ to any tools that use JSON Schema.
Mutating the schema with `replace` is a destructive operation that will interfere with schema-based tooling that expects schemas to be referenced by their canonical URI, for example for the purpose of identifying which "types" are the same.
Catalogs should be handled right above the transport layer, and not be observable in any schema-related behaviors or layers above it.
