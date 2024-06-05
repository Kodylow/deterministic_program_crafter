{
  description = ''
    REPLACE-ME-WITH-CRATE-DESCRIPTION
  '';

  inputs = {
    nixpkgs = { url = "github:nixos/nixpkgs/nixos-23.11"; };

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flakebox = {
      url = "github:dpc/flakebox?rev=226d584e9a288b9a0471af08c5712e7fac6f87dc";
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
        packageName = "REPLACE-ME-WITH-CRATE-BINARY-NAME";
        flakeboxLib = flakebox.lib.${system} { };
        rustSrc = flakeboxLib.filterSubPaths {
          root = builtins.path {
            name = packageName;
            path = ./.;
          };
          paths = [ "Cargo.toml" "Cargo.lock" ".cargo" "src" packageName ];
        };

        toolchainArgs = let llvmPackages = pkgs.llvmPackages_11;
        in {
          extraRustFlags = "--cfg tokio_unstable";

          components = [ "rustc" "cargo" "clippy" "rust-analyzer" "rust-src" ];

          args = {
            nativeBuildInputs = [ ]
              ++ lib.optionals (!pkgs.stdenv.isDarwin) [ ];
          };
        } // lib.optionalAttrs pkgs.stdenv.isDarwin {
          # on Darwin newest stdenv doesn't seem to work
          # linking rocksdb
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
          buildInputs = [ pkgs.pkg-config pkgs.openssl ]
            ++ lib.optionals pkgs.stdenv.isDarwin
            [ pkgs.darwin.apple_sdk.frameworks.SystemConfiguration ];
          nativeBuildInputs = [ pkgs.pkg-config ];
        };
        outputs = (flakeboxLib.craneMultiBuild { toolchains = toolchainsStd; })
          (craneLib':
            let
              craneLib = (craneLib'.overrideArgs {
                pname = packageName;
                src = rustSrc;
              }).overrideArgs commonArgs;
            in rec {
              workspaceDeps = craneLib.buildWorkspaceDepsOnly { };
              workspaceBuild =
                craneLib.buildWorkspace { cargoArtifacts = workspaceDeps; };
              package = craneLib.buildPackageGroup {
                pname = packageName;
                packages = [ packageName ];
                mainProgram = packageName;
              };
            });
      in {
        legacyPackages = outputs;
        packages = { default = outputs.package; };
        devShells = flakeboxLib.mkShells {
          packages = [ ];
          buildInputs = commonArgs.buildInputs;
          nativeBuildInputs = [ commonArgs.nativeBuildInputs ];
          shellHook = ''
            export RUSTFLAGS="--cfg tokio_unstable"
            export RUSTDOCFLAGS="--cfg tokio_unstable"
            export RUST_LOG="info"
          '';
        };
      });
}
