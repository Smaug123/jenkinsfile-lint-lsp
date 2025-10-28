{
  description = "Jenkinsfile Language Server - LSP server for validating Jenkinsfiles";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            cargo
            rustc
            rust-analyzer
            clippy
            rustfmt

            # Build dependencies
            pkg-config
            openssl

            # Development tools
            curl
            jq
          ];

          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
        };

        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "jenkinsfile-ls";
          version = "0.1.0";
          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];

          buildInputs = with pkgs; [
            openssl
          ];

          meta = with pkgs.lib; {
            description = "LSP server for validating Jenkinsfiles via remote Jenkins instance";
            homepage = "https://github.com/yourusername/jenkinsfile-ls";
            license = licenses.mit;
            maintainers = [ ];
          };
        };
      }
    );
}
