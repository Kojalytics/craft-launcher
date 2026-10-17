//! Kojalytics license keys and activation certificates.
//!
//! A key (`KOJA-XXXXX-XXXXX-XXXXX-XXXXX`) is activated once at kojalytics.com, which answers with a
//! certificate: `base64url(JSON payload) "." base64url(Ed25519 signature of the payload part)`,
//! signed with Kojalytics' private key. The launcher keeps the certificate and checks it offline
//! against [`PUBLIC_KEY`] and this machine's device hash.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ed25519_dalek::{Signature, VerifyingKey};
use serde::Deserialize;

/// Kojalytics' certificate-signing public key (Ed25519, raw).
pub const PUBLIC_KEY: [u8; 32] = [
    0x7a, 0x55, 0x81, 0x5a, 0xb1, 0xae, 0xda, 0xdc, 0xd7, 0x34, 0x0d, 0xcd, 0x52, 0xf4, 0x3a, 0x6c, 0xf9, 0x8d, 0x74, 0xcb, 0x7f, 0x7f,
    0x64, 0xab, 0x0b, 0x1b, 0x16, 0x22, 0x00, 0xc2, 0xdc, 0x14,
];

/// Where keys are activated.
pub const ACTIVATE_URL: &str = "https://kojalytics.com/api/license/activate";

/// Crockford base32: no I, L, O or U.
const ALPHABET: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";
const BODY_LENGTH: usize = 20;

/// What a valid certificate says.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct License {
    pub license: String,
    /// The key's last five characters, to tell keys apart.
    pub hint: String,
    pub device: String,
    /// Unix seconds.
    pub issued: i64,
}

/// `koja-abcde …` → `ABCDE…` (20 characters), or `None` when it can't be a Koja key. Typing
/// slips are forgiven: case, spaces and dashes, and O/I/L for 0/1/1.
pub fn normalize_key(input: &str) -> Option<String> {
    let mut s: String = input.chars().filter(|c| !c.is_whitespace() && *c != '-').collect::<String>().to_uppercase();
    if let Some(rest) = s.strip_prefix("KOJA") {
        s = rest.to_string();
    }
    let s: String = s.chars().map(|c| match c {
        'O' => '0',
        'I' | 'L' => '1',
        c => c,
    }).collect();
    (s.chars().count() == BODY_LENGTH && s.chars().all(|c| ALPHABET.contains(c))).then_some(s)
}

/// Checks `certificate`'s signature and that it was issued for `device`.
pub fn verify(certificate: &str, device: &str) -> Result<License, String> {
    verify_with(certificate, device, &PUBLIC_KEY)
}

fn verify_with(certificate: &str, device: &str, public_key: &[u8; 32]) -> Result<License, String> {
    let (payload, signature) = certificate.trim().split_once('.').ok_or("not a license certificate")?;
    let signature = URL_SAFE_NO_PAD.decode(signature).map_err(|_| "not a license certificate")?;
    let signature = Signature::from_slice(&signature).map_err(|_| "not a license certificate")?;
    let key = VerifyingKey::from_bytes(public_key).map_err(|_| "invalid public key")?;
    key.verify_strict(payload.as_bytes(), &signature).map_err(|_| "the license certificate isn't signed by Kojalytics")?;
    let json = URL_SAFE_NO_PAD.decode(payload).map_err(|_| "unreadable license certificate")?;
    let license: License = serde_json::from_slice(&json).map_err(|_| "unreadable license certificate")?;
    if license.device != device {
        return Err("the license was activated on another device".into());
    }
    Ok(license)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn certificate(signer: &SigningKey, device: &str) -> String {
        let payload = URL_SAFE_NO_PAD.encode(format!(r#"{{"v":1,"license":"abc","hint":"STVWX","device":"{device}","issued":1792000000}}"#));
        let signature = signer.sign(payload.as_bytes());
        format!("{payload}.{}", URL_SAFE_NO_PAD.encode(signature.to_bytes()))
    }

    #[test]
    fn keys_are_normalized_forgivingly() {
        assert_eq!(normalize_key("KOJA-ABCDE-FGHJK-MNPQR-STVWX").as_deref(), Some("ABCDEFGHJKMNPQRSTVWX"));
        assert_eq!(normalize_key(" koja abcde fghjk mnpqr stvwx ").as_deref(), Some("ABCDEFGHJKMNPQRSTVWX"));
        assert_eq!(normalize_key("ABCDE-FGHJK-MNPQR-STVWO").as_deref(), Some("ABCDEFGHJKMNPQRSTVW0"));
        assert_eq!(normalize_key("KOJA-ABCDE-FGHJK-MNPQR"), None);
        assert_eq!(normalize_key("KOJA-ABCDE-FGHJK-MNPQR-STVWU"), None);
    }

    #[test]
    fn accepts_only_a_signed_certificate_for_this_device() {
        let signer = SigningKey::from_bytes(&[7; 32]);
        let public = signer.verifying_key().to_bytes();
        let cert = certificate(&signer, "dev1");
        let license = verify_with(&cert, "dev1", &public).unwrap();
        assert_eq!((license.license.as_str(), license.hint.as_str(), license.issued), ("abc", "STVWX", 1792000000));
        assert!(verify_with(&cert, "dev2", &public).is_err());
        let other = SigningKey::from_bytes(&[8; 32]);
        assert!(verify_with(&certificate(&other, "dev1"), "dev1", &public).is_err());
        let (payload, sig) = cert.split_once('.').unwrap();
        let tampered = format!("{}.{sig}", payload.replace('A', "B"));
        assert!(verify_with(&tampered, "dev1", &public).is_err());
        assert!(verify_with("garbage", "dev1", &public).is_err());
    }

    #[test]
    fn the_built_in_key_is_a_valid_ed25519_key() {
        assert!(VerifyingKey::from_bytes(&PUBLIC_KEY).is_ok());
    }
}
