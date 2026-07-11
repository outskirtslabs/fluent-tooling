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
    let
      ftlLintPackage =
        pkgs:
        pkgs.rustPlatform.buildRustPackage {
          pname = "ftl-lint";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          cargoBuildFlags = [
            "-p"
            "fluent-lint"
            "--bin"
            "ftl-lint"
          ];
          cargoTestFlags = [ "--workspace" ];
          meta = {
            description = "Linter for Fluent FTL";
            homepage = "https://github.com/outskirtslabs/fluent-tooling";
            license = pkgs.lib.licenses.mit;
            mainProgram = "ftl-lint";
          };
        };

      fluentGrammarPackage =
        pkgs:
        pkgs.tree-sitter.buildGrammar {
          language = "fluent";
          version = "0.1.0";
          src = ./.;
          postInstall = ''
            mkdir -p "$out/lib"
            ln -s ../parser \
              "$out/lib/libtree-sitter-fluent${pkgs.stdenv.hostPlatform.extensions.sharedLibrary}"
          '';
          meta = {
            description = "Tree-sitter grammar for Fluent FTL";
            homepage = "https://github.com/outskirtslabs/fluent-tooling";
            license = pkgs.lib.licenses.mit;
          };
        };
    in
    devenv.lib.mkFlake ./. {
      inherit inputs;
      withOverlays = [
        devshell.overlays.default
        devenv.overlays.default
      ];
      packages = {
        default = ftlLintPackage;
        ftl-lint = ftlLintPackage;
        tree-sitter-fluent = fluentGrammarPackage;
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
            ((pkgs.emacsPackagesFor pkgs.emacs).emacsWithPackages (epkgs: [
              epkgs.flycheck
            ]))
            pkgs.pkg-config
            pkgs.rustc
          ];
        };
    };
}
