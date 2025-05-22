/**
  Construct a Nix library (attrset of functions) from a Nixpkgs instance.

  # Inputs

  - `pkgs`: A Nixpkgs instance, e.g. `import <nixpkgs> {}`.
*/
{
  pkgs,
  lib ? pkgs.lib,
  json-schema-catalog-rs,
  ...
}:
let
  jsonFormat = pkgs.formats.json { };
in
{
  /**
    The `json-schema-catalog-rs` package to use, as configured when instantiating
    this library.
  */
  inherit json-schema-catalog-rs;

  /**
    A somewhat opinionated method for constructing a JSON Schema Catalog from
    files in a Nix store.

    The input is a simpler format:

    ```nix
    {
      name = "my-catalog"; # derivation name, default displayName, no slashes
      displayName = "My Catalog"; # optional
      groups = {
        "Group One" = {
          "https://example.com/schemas/one-v1.json" = pkgs.fetchurl { ... };
          "https://example.com/schemas/one-v2.json" = pkgs.fetchurl { ... };
          "https://example.com/schemas/one-common.json" = pkgs.fetchurl { ... };
        };
        "Group Two" = {
          "https://example.com/schemas/two-v1.json" = ./two-v1.json; # Files can be local
        };
      };
    }
    ```
  */
  newCatalog =
    {
      name,
      displayName ? name,
      groups,
    }:
    pkgs.runCommand "catalog-${name}"
      {
        catalogJson = builtins.toJSON {
          name = displayName;
          groups = lib.mapAttrsToList (name: group: {
            inherit name;
            # TODO dedup the longest common prefix by putting it in baseLocation
            baseLocation = "/";
            schemas = lib.mapAttrsToList (id: location: {
              inherit id;
              inherit location;
            }) group;
          }) groups;
        };
        passAsFile = [ "catalogJson" ];
        passthru = {
          inherit groups;
        };
        nativeBuildInputs = [
          pkgs.jq
          json-schema-catalog-rs
        ];
      }
      ''
        out_dir="$out/share/json-schema-catalogs"
        out_file="$out_dir/$name.json"
        mkdir -p "$out_dir"
        jq . <"$catalogJsonPath" >"$out_file"
        json-schema-catalog check "$out_file"
      '';

}
