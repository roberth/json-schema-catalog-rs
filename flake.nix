{
  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  inputs.nci.url = "github:yusdacra/nix-cargo-integration";
  inputs.nci.inputs.nixpkgs.follows = "nixpkgs";
  inputs.parts.url = "github:hercules-ci/flake-parts";
  inputs.parts.inputs.nixpkgs-lib.follows = "nixpkgs";

  outputs = inputs @ {
    parts,
    nci,
    ...
  }:
    parts.lib.mkFlake {inherit inputs;} {
      systems = ["x86_64-linux"];
      imports = [
        nci.flakeModule
      ];
      perSystem = {
        pkgs,
        config,
        ...
      }: let
        # shorthand for accessing this crate's outputs
        # you can access crate outputs under `config.nci.outputs.<crate name>` (see documentation)
        crateOutputs = config.nci.outputs."json-schema-catalog-rs";
      in {
        # export the crate devshell as the default devshell
        devShells.default = crateOutputs.devShell.overrideAttrs (prevAttrs: {
          nativeBuildInputs = prevAttrs.nativeBuildInputs;
        });
        # export the release package of the crate as default package
        packages.default = crateOutputs.packages.release;

        checks = import ./pkgs-lib-tests.nix {
          inherit pkgs;
          jsonSchemaCatalogLib = import ./pkgs-lib.nix {
            inherit pkgs;
            json-schema-catalog-rs = config.packages.default;
          };
        };

        # nix-cargo-integration:
        # https://flake.parts/options/nix-cargo-integration
        # https://github.com/yusdacra/nix-cargo-integration#readme

        # declare projects
        nci.projects."json-schema-catalog-rs".path = ./.;
        # configure crates
        nci.crates."json-schema-catalog-rs" = {};
      };

      flake.lib = {
        withPkgs = { pkgs, ... }: import ./pkgs-lib.nix { inherit pkgs; };
      };
    };
}
