{
  description = "TabletBot";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs, ... }: let

    system = "x86_64-linux";

    importPkgs = pkgs: import pkgs { inherit system; };

    pkgs = import nixpkgs {
      inherit system;
      config.allowUnfree = true;
    };

  in rec {

    packages.${system} = rec {
      tabletbot = pkgs.callPackage ./default.nix { flake = self; };
      default = tabletbot;
    };

    devShells.${system}.default = import ./shell.nix {
      inherit pkgs;
      packages = packages.${system};
    };
  };
}
