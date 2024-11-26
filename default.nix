{ lib
, rustPlatform
, pkg-config
, openssl
, fetchurl
, ...
}:

rustPlatform.buildRustPackage rec {
  pname = "tabletbot";
  name = pname;

  nativeBuildInputs = [
    pkg-config
  ];

  buildInputs = [
    openssl
  ];

  src = ./.;

  cargoLock = {
    lockFile = ./Cargo.lock;
    allowBuiltinFetchGit = true;

  };
}
