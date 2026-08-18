{
  description = "qitech_lib";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { nixpkgs, rust-overlay, ... }:
  let
    system = "x86_64-linux";
    pkgs = import nixpkgs {
      inherit system;
      overlays = [ rust-overlay.overlays.default ];
    };
    rust = pkgs.rust-bin.stable.latest.default.override {
      extensions = [ "rust-src" "rust-analyzer" "clippy" ];
    };
  in {
    devShells.${system}.default = pkgs.mkShell {
      packages = [
        rust
        pkgs.pkg-config
        pkgs.openssl
      ];

      env = {
        RUST_BACKTRACE = "1";
      };
    };
  };
}

