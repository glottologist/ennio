{
  description = "Ennio — AI agent orchestrator";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, crane, rust-overlay, flake-utils, advisory-db }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        inherit (pkgs) lib;

        rustToolchain = pkgs.rust-bin.stable."1.88.0".default.override {
          targets = [ "wasm32-unknown-unknown" ];
        };

        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        src = lib.fileset.toSource {
          root = ./.;
          fileset = lib.fileset.unions [
            (craneLib.fileset.commonCargoSources ./.)
            (lib.fileset.fileFilter (file: file.hasExt "proto") ./.)
          ];
        };

        commonArgs = {
          inherit src;
          strictDeps = true;
          pname = "ennio-workspace";
          version = "0.1.0";

          nativeBuildInputs = with pkgs; [
            pkg-config
            protobuf
          ];

          PROTOC = "${pkgs.protobuf}/bin/protoc";

          buildInputs = with pkgs; [
          ] ++ lib.optionals stdenv.isDarwin [
            libiconv
            darwin.apple_sdk.frameworks.Security
            darwin.apple_sdk.frameworks.SystemConfiguration
          ];
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        ennio = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          pname = "ennio";
          cargoExtraArgs = "-p ennio-cli";
        });

        ennio-node = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          pname = "ennio-node";
          cargoExtraArgs = "-p ennio-node";
        });

        wasmCommonArgs = commonArgs // {
          pname = "ennio-dashboard";
          cargoExtraArgs = "-p ennio-dashboard";
          CARGO_BUILD_TARGET = "wasm32-unknown-unknown";
          doCheck = false;
        };

        cargoArtifactsWasm = craneLib.buildDepsOnly wasmCommonArgs;

        ennio-dashboard = craneLib.buildPackage (wasmCommonArgs // {
          cargoArtifacts = cargoArtifactsWasm;
        });

      in
      {
        checks = {
          inherit ennio ennio-node ennio-dashboard;

          clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          });

          fmt = craneLib.cargoFmt {
            inherit src;
          };

          nextest = craneLib.cargoNextest (commonArgs // {
            inherit cargoArtifacts;
          });

          doc = craneLib.cargoDoc (commonArgs // {
            inherit cargoArtifacts;
            RUSTDOCFLAGS = "--deny warnings";
          });

          audit = craneLib.cargoAudit {
            inherit src advisory-db;
          };

          toml-fmt = craneLib.taploFmt {
            src = lib.fileset.toSource {
              root = ./.;
              fileset = lib.fileset.fileFilter (file: file.hasExt "toml") ./.;
            };
          };
        };

        packages = {
          inherit ennio ennio-node ennio-dashboard;
          default = ennio;
        };

        apps = {
          default = flake-utils.lib.mkApp { drv = ennio; };
          ennio-node = flake-utils.lib.mkApp { drv = ennio-node; };
        };

        devShells.default = craneLib.devShell {
          checks = self.checks.${system};
          packages = with pkgs; [
            cargo-nextest
            cargo-audit
            cargo-watch
            cargo-bloat
            bacon
            rust-analyzer
          ];
          RUST_BACKTRACE = "1";
          RUST_LOG = "debug";
        };
      }
    ) // {
      overlays.default = final: prev: {
        ennio = self.packages.${final.system}.ennio;
        ennio-node = self.packages.${final.system}.ennio-node;
      };
    };
}
