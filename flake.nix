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
      pkgs.cargo-sweep
      pkgs.pkg-config
      pkgs.openssl
      pkgs.jq
    ];

    craneLibFor = system: let
      pkgs = pkgsFor system;
      toolchain = pkgs.rust-bin.stable.latest.default;
    in (crane.mkLib pkgs).overrideToolchain toolchain;

    commonArgsFor = system: let
      pkgs = pkgsFor system;
    in {
      src = (craneLibFor system).cleanCargoSource ./.;
      buildInputs = pkgs.lib.optionals pkgs.stdenv.isDarwin [
        pkgs.darwin.apple_sdk.frameworks.Security
        pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
      ];
      nativeBuildInputs = [ pkgs.pkg-config ];
    };
  in {

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
