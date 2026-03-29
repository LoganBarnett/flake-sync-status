{
  description = "Check whether NixOS/nix-darwin hosts are in sync with a flake";
  inputs = {
    crane.url = "github:ipetkov/crane";
    nixpkgs.url = github:NixOS/nixpkgs/25.11;
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, rust-overlay, crane }@inputs: let
    forAllSystems = nixpkgs.lib.genAttrs nixpkgs.lib.systems.flakeExposed;
    overlays = [ (import rust-overlay) ];
    pkgsFor = system: import nixpkgs {
      inherit system;
      overlays = overlays;
    };

    workspaceCrates = {
      # Note: The 'lib' crate is not included here as it doesn't produce a binary.
      cli = {
        name = "flake-sync-status";
        binary = "flake-sync-status";
        description = "Report sync status of flake hosts vs. their active generations";
      };
    };

    devPackages = pkgs: let
      rust = pkgs.rust-bin.stable.latest.default.override {
        extensions = [
          # For rust-analyzer and others.  See
          # https://nixos.wiki/wiki/Rust#Shell.nix_example for some details.
          "rust-src"
          "rust-analyzer"
          "rustfmt"
        ];
      };
    in [
      rust
      pkgs.alejandra
      pkgs.cargo-deny
      pkgs.cargo-sweep
      pkgs.jq
      pkgs.openssl
      pkgs.pkg-config
      pkgs.treefmt
    ];

    craneLibFor = system: let
      pkgs = pkgsFor system;
      toolchain = pkgs.rust-bin.stable.latest.default;
    in (crane.mkLib pkgs).overrideToolchain toolchain;

    commonArgsFor = system: let
      pkgs = pkgsFor system;
    in {
      pname = "flake-sync-status";
      src = (craneLibFor system).cleanCargoSource ./.;
      nativeBuildInputs = [ pkgs.pkg-config ];
      # Run unit tests only.  Integration tests in tests/ invoke the compiled
      # binary directly, which is not available in the Nix sandbox.
      cargoTestExtraArgs = "--lib --bins";
    };
  in {

    checks = forAllSystems (system: let
      craneLib = craneLibFor system;
      args = commonArgsFor system;
      pkgs = pkgsFor system;
    in {
      # License, ban, and source-policy checks via cargo-deny.
      # Advisory checks are omitted here because they require fetching the
      # RustSec advisory DB, which is unavailable in the Nix build sandbox.
      # Run `cargo deny check advisories` manually or in CI.
      deny = craneLib.mkCargoDerivation (args // {
        pname = "flake-sync-status-deny";
        cargoArtifacts = null;
        doInstallCargoArtifacts = false;
        buildPhaseCargoCommand =
          "cargo deny check licenses bans sources";
        nativeBuildInputs = (args.nativeBuildInputs or []) ++ [ pkgs.cargo-deny ];
      });
    });

    packages = forAllSystems (system: {
      default = (craneLibFor system).buildPackage (commonArgsFor system);
      flake-sync-status = self.packages.${system}.default;
    });

    apps = forAllSystems (system: {
      default = {
        type = "app";
        program = "${self.packages.${system}.default}/bin/flake-sync-status";
      };
    });

    nixosModules.default = import ./nix/modules/nixos.nix;
    darwinModules.default = import ./nix/modules/darwin.nix;

    devShells = forAllSystems (system: {
      default = (pkgsFor system).mkShell {
        buildInputs = devPackages (pkgsFor system);
        shellHook = ''
          echo "flake-sync-status development environment"
          echo ""
          echo "Available Cargo packages (use 'cargo build -p <name>'):"
          cargo metadata --no-deps --format-version 1 2>/dev/null | \
            jq -r '.packages[].name' | \
            sort | \
            sed 's/^/  • /' || echo "  Run 'cargo init' to get started"
        '';
      };
    });

  };
}
