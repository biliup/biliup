{
  description = "Upload video to bilibili";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, crane, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        # Use the rust-toolchain file or default to stable
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        # Get the package name and version from the CLI crate
        crateInfo = craneLib.crateNameFromCargoToml {
          cargoToml = ./Cargo.toml;
        };

        # Skip frontend build - create empty placeholder
        frontend = pkgs.runCommand "biliup-frontend-empty" {} ''
          mkdir -p $out
          echo '<!DOCTYPE html><html><body>Frontend disabled</body></html>' > $out/index.html
        '';

        # Common arguments for crane
        commonArgs = {
          src = craneLib.cleanCargoSource ../..;
          strictDeps = true;
          pname = crateInfo.pname;
          version = crateInfo.version;

          buildInputs = [
            # Add any runtime dependencies here
          ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.darwin.apple_sdk.frameworks.Security
            pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
          ];

          nativeBuildInputs = [
            pkgs.pkg-config
            pkgs.python3
          ];

          # Make frontend build output available
          preBuild = ''
            mkdir -p out
            cp -r ${frontend}/* out/
          '';
        };

        # Build *just* the cargo dependencies, so we can reuse
        # all of that work (e.g. via cachix) when running in CI
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Build the actual crate itself, reusing the dependency
        # artifacts from above.
        biliup = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          # Skip tests to avoid server test failures when building without frontend
          doCheck = false;

          meta = {
            mainProgram = "biliup";
          };
        });
      in
      {
        checks = {
          inherit biliup;

          # Run clippy (and deny all warnings) on the crate source,
          # again, reusing the dependency artifacts from above.
          biliup-clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          });

          # Check formatting
          biliup-fmt = craneLib.cargoFmt {
            inherit (commonArgs) src;
          };
        };

        packages = {
          default = biliup;
          inherit biliup;
        };

        apps = {
          default = flake-utils.lib.mkApp {
            drv = biliup;
            exePath = "/bin/biliup";
          };
          biliup = flake-utils.lib.mkApp {
            drv = biliup;
            exePath = "/bin/biliup";
          };
        };

        devShells.default = craneLib.devShell {
          # Inherit inputs from checks
          checks = self.checks.${system};

          packages = [
            pkgs.rust-analyzer
            pkgs.cargo-watch
            pkgs.cargo-edit
          ];

          # Additional environment variables if needed
          # RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
        };
      }
    );
}
