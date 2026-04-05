{ lib, buildGoModule, fetchFromGitHub, sqlite }:

buildGoModule rec {
  pname = "blossom-server";
  version = "2.0.0";

  src = fetchFromGitHub {
    owner = "sebdeveloper6952";
    repo = "blossom-server";
    rev = "v${version}";
    hash = "sha256-koTof+RnGVXh1FkDQb70K6o0VbT/ghz8WpIBCHEVT9g=";
  };

  vendorHash = "sha256-MQ7mEGCjTq22Pb+PpUMaVehXHt2BKy4DZ4xVb2Px3xA=";

  env.CGO_ENABLED = "1";
  buildInputs = [ sqlite ];

  subPackages = [ "cmd/api" ];

  postInstall = ''
    mv $out/bin/api $out/bin/blossom-server

    # Migrations are loaded at runtime from db/migrations relative to CWD
    mkdir -p $out/share/blossom-server
    cp -r $src/db $out/share/blossom-server/db
  '';

  meta = {
    description = "Blossom media server (BUD protocol) for Nostr";
    homepage = "https://github.com/sebdeveloper6952/blossom-server";
    license = lib.licenses.mit;
    mainProgram = "blossom-server";
  };
}
