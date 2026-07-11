{
  description = "Tree-sitter grammar and lint tooling for Fluent FTL";
  inputs = {
    nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/0.1"; # tracks nixpkgs unstable branch
    devshell.url = "github:numtide/devshell";
    devshell.inputs.nixpkgs.follows = "nixpkgs";
    devenv.url = "https://flakehub.com/f/ramblurr/nix-devenv/*";
    devenv.inputs.nixpkgs.follows = "nixpkgs";
  };
  outputs =
    inputs@{
      self,
      devenv,
      devshell,
      ...
    }:
    devenv.lib.mkFlake ./. {
      inherit inputs;
      withOverlays = [
        devshell.overlays.default
        devenv.overlays.default
      ];
      packages.default =
        pkgs:
        pkgs.rustPlatform.buildRustPackage {
          pname = "fluent-tooling";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          cargoBuildFlags = [
            "-p"
            "fluent-lint"
            "--bin"
            "fl-lint"
          ];
          cargoTestFlags = [ "--workspace" ];
          meta = {
            description = "Tree-sitter grammar and linter for Fluent FTL";
            homepage = "https://github.com/outskirtslabs/fluent-tooling";
            license = pkgs.lib.licenses.mit;
            mainProgram = "fl-lint";
          };
        };
      devShell =
        pkgs:
        pkgs.devshell.mkShell {
          imports = [
            devenv.capsules.base
          ];
          # https://numtide.github.io/devshell
          commands = [
            { package = pkgs.babashka; }
            { package = pkgs.cargo; }
            { package = pkgs.clippy; }
            { package = pkgs.prettier; }
            { package = pkgs.rustfmt; }
            { package = pkgs.tree-sitter; }
          ];
          packages = [
            pkgs.clang
            pkgs.pkg-config
            pkgs.rustc
          ];
        };
    };
}
