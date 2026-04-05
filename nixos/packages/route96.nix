{ lib, rustPlatform, fetchgit, pkg-config, openssl, ffmpeg, libclang, llvmPackages }:

rustPlatform.buildRustPackage rec {
  pname = "route96";
  version = "0.6.1";

  src = fetchgit {
    url = "https://git.v0l.io/Kieran/route96.git";
    rev = "v${version}";
    hash = "sha256-C6OnzLrDKJbHNH/J2WaUm66ylBEgkCXzLMDqeQwpNEc=";
    fetchSubmodules = false;
  };

  cargoHash = "sha256-B0lL3iAAvAmr1kJek3RqXrO3/fqqnXvpGpLRpwrQt3A=";

  buildNoDefaultFeatures = true;
  buildFeatures = [ "blossom" "nip96" "media-compression" "r96util" ];

  nativeBuildInputs = [ pkg-config llvmPackages.clang ];
  buildInputs = [ openssl ffmpeg ];

  LIBCLANG_PATH = "${libclang.lib}/lib";

  # sqlx migrations are embedded at compile time
  SQLX_OFFLINE = "true";

  # phash tests require ffmpeg in PATH at test time
  doCheck = false;

  meta = {
    description = "route96 — Blossom/NIP-96 media server for Nostr";
    homepage = "https://git.v0l.io/Kieran/route96";
    license = lib.licenses.mit;
    mainProgram = "route96";
  };
}
