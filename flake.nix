{
  description = "Bitcoin Data MCP Server";

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

        # Import the rust-toolchain.toml
        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

        buildInputs = [ rustToolchain ];

      in
      {
        devShells.default = pkgs.mkShell {
          inherit buildInputs;
        };

        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "memory-mcp";
          version = "0.1.0";

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          inherit buildInputs;

          meta = with pkgs.lib; {
            description = "Bitcoin Data MCP Server";
          };
        };

        apps.default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/memory-mcp";
        };
      }
    );
}

