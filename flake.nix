{
  description = "A tool for initializing new projects";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs,flake-utils }: flake-utils.lib.eachDefaultSystem (system: 
    let pkgs = nixpkgs.legacyPackages.${system}; in
  {
    packages.initx = pkgs.rustPlatform.buildRustPackage {
      pname = "initx";
      version = "1.0";
      src = ./.;
      cargoLock.lockFile = ./Cargo.lock;
    };

    packages.default = self.packages.${system}.initx;
  });
}
