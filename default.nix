{ lib
, rustPlatform
, pkg-config
, openssl
, fetchurl
, aws-lc
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
    aws-lc
  ];

  src = ./.;

  cargoLock = {
    lockFile = ./Cargo.lock;
    allowBuiltinFetchGit = true;

  };
}
