{ lib, rustPlatform, fetchgit, pkg-config, openssl, ffmpeg, libclang, llvmPackages }:

rustPlatform.buildRustPackage {
  pname = "route96";
  version = "0.6.1";

  src = fetchgit {
    url = "https://git.v0l.io/Kieran/route96.git";
    rev = "v0.6.1";
    hash = "sha256-C6OnzLrDKJbHNH/J2WaUm66ylBEgkCXzLMDqeQwpNEc=";
    fetchSubmodules = false;
  };

  cargoHash = "sha256-B0lL3iAAvAmr1kJek3RqXrO3/fqqnXvpGpLRpwrQt3A=";

  buildNoDefaultFeatures = true;
  buildFeatures = [ "blossom" "nip96" "media-compression" "r96util" ];

  nativeBuildInputs = [ pkg-config llvmPackages.clang ];
  buildInputs = [ openssl ffmpeg ];

  LIBCLANG_PATH = "${libclang.lib}/lib";
  SQLX_OFFLINE = "true";
  doCheck = false;

  meta = {
    description = "route96 — Blossom/NIP-96 media server for Nostr";
    homepage = "https://git.v0l.io/Kieran/route96";
    license = lib.licenses.mit;
    mainProgram = "route96";
  };
}
