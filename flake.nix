{
  description = "Tenchu: Shinobi Gaisen portable Rust PC port";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/9ff9f00c0c1a903d1d938a0dd31b8eacf8d00adc";

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      eachSystem = f: nixpkgs.lib.genAttrs systems (system: f (import nixpkgs { inherit system; }));
      source =
        lib:
        lib.fileset.toSource {
          root = ./.;
          fileset = lib.fileset.unions [
            ./Cargo.toml
            ./Cargo.lock
            ./crates
          ];
        };
      native =
        pkgs:
        pkgs.rustPlatform.buildRustPackage {
          pname = "tenchu-shinobi-gaisen-pc";
          version = "0.1.0";
          src = source pkgs.lib;
          cargoLock.lockFile = ./Cargo.lock;
          meta.mainProgram = "tenchu-pc";
        };
    in
    {
      packages = eachSystem (pkgs: {
        default = native pkgs;
      });
      apps = eachSystem (pkgs: {
        default = {
          type = "app";
          meta.description = "Tenchu: Shinobi Gaisen PC port";
          program = "${native pkgs}/bin/tenchu-pc";
        };
      });
      devShells = eachSystem (pkgs: {
        default = pkgs.mkShell {
          TENCHU_DEV_SHELL = "1";
          packages = with pkgs; [
            cargo
            rustc
            rustfmt
            clippy
            rust-analyzer
            gcc
            lld
            nixfmt
          ];
          CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_LINKER = "${pkgs.lld}/bin/wasm-ld";
        };
      });
      checks = eachSystem (pkgs: {
        native = native pkgs;
        wasm = pkgs.rustPlatform.buildRustPackage {
          pname = "tenchu-portable-core";
          version = "0.1.0";
          src = source pkgs.lib;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = [ pkgs.lld ];
          CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_LINKER = "${pkgs.lld}/bin/wasm-ld";
          buildPhase = ''
            cargo build --frozen --offline --release --target wasm32-unknown-unknown \
              -p tenchu-assets -p tenchu-core --lib
          '';
          doCheck = false;
          installPhase = ''
            mkdir -p "$out/lib"
            cp target/wasm32-unknown-unknown/release/libtenchu_*.rlib "$out/lib/"
          '';
        };
        formatting =
          pkgs.runCommand "tenchu-format"
            {
              nativeBuildInputs = [
                pkgs.cargo
                pkgs.rustfmt
              ];
            }
            ''
              cd ${source pkgs.lib}
              cargo fmt --all -- --check
              touch "$out"
            '';
      });
      formatter = eachSystem (pkgs: pkgs.nixfmt);
    };
}
