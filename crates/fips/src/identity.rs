//! Write FIPS key files from NostrBox identity.
//!
//! FIPS reads its keypair from `fips.key` (nsec bech32) and `fips.pub` (npub bech32).
//! NostrBox shares its identity with FIPS so both use the same npub.

use nostr_sdk::ToBech32;
use nostrbox_core::BoxIdentity;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use crate::FipsError;

/// Write FIPS key and pub files from a BoxIdentity.
///
/// - `key_path`: e.g. `/var/lib/nostrbox/fips.key` (mode 0600)
/// - `pub_path`: e.g. `/var/lib/nostrbox/fips.pub` (mode 0644)
pub fn write_fips_key_files(
    identity: &BoxIdentity,
    key_path: &Path,
    pub_path: &Path,
) -> Result<(), FipsError> {
    let keys = identity.keys();

    // Write nsec (secret key) — restricted permissions
    let nsec = keys
        .secret_key()
        .to_bech32()
        .map_err(|e| FipsError::Identity(format!("failed to encode nsec: {e}")))?;
    fs::write(key_path, format!("{nsec}\n"))
        .map_err(|e| FipsError::Identity(format!("failed to write {}: {e}", key_path.display())))?;
    fs::set_permissions(key_path, fs::Permissions::from_mode(0o600))
        .map_err(|e| FipsError::Identity(format!("failed to chmod {}: {e}", key_path.display())))?;

    // Write npub (public key) — world-readable
    let npub = keys
        .public_key()
        .to_bech32()
        .map_err(|e| FipsError::Identity(format!("failed to encode npub: {e}")))?;
    fs::write(pub_path, format!("{npub}\n"))
        .map_err(|e| FipsError::Identity(format!("failed to write {}: {e}", pub_path.display())))?;

    tracing::info!(
        npub = %npub,
        key_path = %key_path.display(),
        "wrote FIPS identity files"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_write_key_files() {
        let identity = BoxIdentity::from_nsec(
            "nsec1vl029mgpspedva04g90vltkh6fvh240zqtv9k0t9af8935ke9laqsnlfe5",
        )
        .unwrap();

        let dir = TempDir::new().unwrap();
        let key_path = dir.path().join("fips.key");
        let pub_path = dir.path().join("fips.pub");

        write_fips_key_files(&identity, &key_path, &pub_path).unwrap();

        let key_contents = fs::read_to_string(&key_path).unwrap();
        assert!(key_contents.trim().starts_with("nsec1"));

        let pub_contents = fs::read_to_string(&pub_path).unwrap();
        assert!(pub_contents.trim().starts_with("npub1"));

        // Key file should be mode 0600
        let meta = fs::metadata(&key_path).unwrap();
        assert_eq!(meta.permissions().mode() & 0o777, 0o600);
    }
}
