{ pkgs, jsonSchemaCatalogLib, lib ? pkgs.lib, ... }:
rec {
  exampleCatalog = jsonSchemaCatalogLib.newCatalog {
    name = "example";
    displayName = "Example Catalog";
    groups = {
      "JSON Schema" = {
        "https://json-schema.org/draft-07/schema#" =
          pkgs.fetchurl {
            name = "json-schema-draft-07";
            url = "https://json-schema.org/draft-07/schema#";
            sha256 = "sha256-aS4dFl5Hr8tfEbLOHGOWNf+oNANdbstrzzCHSB2uhAQ=";
          };
      };
    };
  };

  integrationTest = pkgs.runCommand "integration-test" {
    nativeBuildInputs = [ exampleCatalog jsonSchemaCatalogLib.json-schema-catalog-rs ];
  } ''
    cat >example.json <<"EOF"
    {
      "$id": "https://example.com/schemas/integration-test.json",
      "$schema": "https://json-schema.org/draft-07/schema#",
      "title": "Integration Test",
      "type": "object"
    }
    EOF
    cat >example.json.expected <<"EOF"
    {
      "$id": "https://example.com/schemas/integration-test.json",
      "$schema": "${exampleCatalog.groups."JSON Schema"."https://json-schema.org/draft-07/schema#"}",
      "title": "Integration Test",
      "type": "object"
    }
    EOF

    json-schema-catalog replace --verbose example.json > example.json.out

    diff -U3 --color=always example.json.expected example.json.out
    touch $out
  '';
}