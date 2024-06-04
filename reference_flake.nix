{
  description = "<REPLACE_ME_WITH_DESCRIPTION>";

  inputs = {
    nixpkgs = { url = "github:nixos/nixpkgs/nixos-23.11"; };

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flakebox = {
      url = "github:dpc/flakebox";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.fenix.follows = "fenix";
    };

    flake-utils.url = "github:numtide/flake-utils";

  };

  outputs = { self, nixpkgs, flakebox, fenix, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        lib = pkgs.lib;
        flakeboxLib = flakebox.lib.${system} { };
        rustSrc = flakeboxLib.filterSubPaths {
          root = builtins.path {
            name = "REPLACE_ME_WITH_CRATE_BINARY_NAME";
            path = ./.;
          };
          paths = [
            "Cargo.toml"
            "Cargo.lock"
            ".cargo"
            "src"
            "REPLACE_ME_WITH_CRATE_BINARY_NAME"
          ];
        };

        toolchainArgs = let llvmPackages = pkgs.llvmPackages_11;
        in {
          extraRustFlags = "REPLACE_ME_WITH_FLAGS";

          components = [ "rustc" "cargo" "clippy" "rust-analyzer" "rust-src" ];

          args = {
            nativeBuildInputs = [ ]
              ++ lib.optionals (!pkgs.stdenv.isDarwin) [ ];
          };
        } // lib.optionalAttrs pkgs.stdenv.isDarwin {
          stdenv = pkgs.clang11Stdenv;
          clang = llvmPackages.clang;
          libclang = llvmPackages.libclang.lib;
          clang-unwrapped = llvmPackages.clang-unwrapped;
        };

        # all standard toolchains provided by flakebox
        toolchainsStd = flakeboxLib.mkStdFenixToolchains toolchainArgs;

        toolchainsNative = (pkgs.lib.getAttrs [ "default" ] toolchainsStd);

        toolchainNative =
          flakeboxLib.mkFenixMultiToolchain { toolchains = toolchainsNative; };

        commonArgs = {
          buildInputs = [ ] ++ lib.optionals pkgs.stdenv.isDarwin
            [ pkgs.darwin.apple_sdk.frameworks.SystemConfiguration ];
          nativeBuildInputs = [ pkgs.pkg-config ];
        };
        outputs = (flakeboxLib.craneMultiBuild { toolchains = toolchainsStd; })
          (craneLib':
            let
              craneLib = (craneLib'.overrideArgs {
                pname = "flexbox-multibuild";
                src = rustSrc;
              }).overrideArgs commonArgs;
            in rec {
              workspaceDeps = craneLib.buildWorkspaceDepsOnly { };
              workspaceBuild =
                craneLib.buildWorkspace { cargoArtifacts = workspaceDeps; };
              REPLACE_ME_WITH_CRATE_BINARY_NAME = craneLib.buildPackageGroup {
                pname = "REPLACE_ME_WITH_CRATE_BINARY_NAME";
                packages = [ "REPLACE_ME_WITH_CRATE_BINARY_NAME" ];
                mainProgram = "REPLACE_ME_WITH_CRATE_BINARY_NAME";
              };
            });
      in {
        legacyPackages = outputs;
        packages = { default = outputs.REPLACE_ME_WITH_CRATE_BINARY_NAME; };
        devShells = flakeboxLib.mkShells {
          packages = [ ];
          buildInputs = [
            pkgs.REPLACE_ME_WITH_DEPENDENCY
            pkgs.REPLACE_ME_WITH_DEPENDENCY
            commonArgs.buildInputs
          ];
          nativeBuildInputs = [
            pkgs.REPLACE_ME_WITH_DEPENDENCY
            pkgs.REPLACE_ME_WITH_DEPENDENCY
            commonArgs.nativeBuildInputs
          ];
          shellHook = ''
            export RUST_LOG="info"
          '';
        };
      });
}
