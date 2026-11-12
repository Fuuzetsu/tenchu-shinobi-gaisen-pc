{
  nixpkgs,
  system,
  architecture ? "aarch64",
}:
let
  pkgs = import nixpkgs {
    inherit system;
    config = {
      allowUnfree = true;
      android_sdk.accept_license = true;
    };
  };
  target = "${architecture}-linux-android";
  cross = import nixpkgs {
    inherit system;
    # NDK r27 contains Clang 18; using the NDK version makes the pinned
    # wrapper add an unsupported x86 TLS dialect flag while building Rust std.
    overlays = [
      (final: prev: {
        wrapCCWith =
          args:
          prev.wrapCCWith (
            args
            // final.lib.optionalAttrs (final.lib.hasInfix "ndk-toolchain" (args.cc.name or "")) {
              cc = args.cc // {
                version = "18.0.0";
              };
            }
          );
      })
    ];
    config = {
      allowUnfree = true;
      android_sdk.accept_license = true;
    };
    crossSystem = {
      config = "${architecture}-unknown-linux-android";
      rust.rustcTarget = target;
      androidSdkVersion = "28";
      androidNdkVersion = "27";
      useAndroidPrebuilt = true;
    };
  };
  sdk = pkgs.androidenv.composeAndroidPackages {
    platformVersions = [ "35" ];
    buildToolsVersions = [ "35.0.0" ];
    includeEmulator = architecture == "x86_64";
  };
  upper = pkgs.lib.toUpper (builtins.replaceStrings [ "-" ] [ "_" ] target);
  targetEnv = builtins.replaceStrings [ "-" ] [ "_" ] target;
  compiler = "${cross.stdenv.cc}/bin/${cross.stdenv.cc.targetPrefix}clang";
in
pkgs.mkShell {
  TENCHU_DEV_SHELL = "1";
  TENCHU_ANDROID_SHELL = "1";
  ANDROID_HOME = "${sdk.androidsdk}/libexec/android-sdk";
  JAVA_HOME = pkgs.jdk17.home;
  CARGO_BUILD_TARGET = target;
  RUSTC = "${cross.buildPackages.rustc}/bin/rustc";
  RUSTDOC = "${cross.buildPackages.rustc}/bin/rustdoc";
  SYSROOT = "${cross.buildPackages.rustc-unwrapped}";
  "CARGO_TARGET_${upper}_LINKER" = compiler;
  "CC_${targetEnv}" = compiler;
  "AR_${targetEnv}" = "${cross.stdenv.cc.bintools}/bin/${cross.stdenv.cc.targetPrefix}ar";
  "CARGO_TARGET_${upper}_RUSTFLAGS" = "-C link-arg=-Wl,-z,max-page-size=16384";
  packages = [
    sdk.androidsdk
    cross.buildPackages.rustc
    pkgs.cargo
    pkgs.clippy
    pkgs.jdk17
    pkgs.python3
    pkgs.zip
    pkgs.unzip
    pkgs.ripgrep
    pkgs.android-tools
    cross.stdenv.cc
    cross.stdenv.cc.bintools
  ];
}
