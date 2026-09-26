//! Strict integrity expectations for shared resource bytes. This supports
//! SHA-256/384/512 SRI digest lists, with strongest-algorithm selection. Malformed
//! or unsupported metadata fails closed instead of disabling verification.
use base64::{
    engine::general_purpose::{STANDARD, STANDARD_NO_PAD},
    Engine,
};
use sha2::{Digest, Sha256, Sha384, Sha512};

pub fn verify_resource_integrity(bytes: &[u8], metadata: &str) -> Result<(), String> {
    let mut digests = Vec::new();
    for token in metadata.split_ascii_whitespace() {
        let (algorithm, value) = token.split_once('-').ok_or("invalid integrity token")?;
        let size = match algorithm {
            "sha256" => 32,
            "sha384" => 48,
            "sha512" => 64,
            _ => return Err("unsupported integrity algorithm".into()),
        };
        let digest = STANDARD
            .decode(value)
            .or_else(|_| STANDARD_NO_PAD.decode(value))
            .map_err(|_| "invalid integrity base64 digest")?;
        if digest.len() != size {
            return Err("invalid integrity digest length".into());
        }
        digests.push((size, digest));
    }
    let strongest = digests
        .iter()
        .map(|(size, _)| *size)
        .max()
        .ok_or("empty integrity expectation")?;
    let actual = match strongest {
        32 => Sha256::digest(bytes).to_vec(),
        48 => Sha384::digest(bytes).to_vec(),
        _ => Sha512::digest(bytes).to_vec(),
    };
    if digests
        .iter()
        .any(|(size, expected)| *size == strongest && *expected == actual)
    {
        Ok(())
    } else {
        Err("resource bytes do not match the strongest integrity digest".into())
    }
}
