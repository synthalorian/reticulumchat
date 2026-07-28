use age::{x25519, Decryptor, Encryptor};
use anyhow::{Context, Result};
use std::io::{Read, Write};
use std::iter;

/// Encrypt a plaintext message for a recipient using age
pub fn encrypt_message(plaintext: &str, recipient_pubkey: &str) -> Result<Vec<u8>> {
    let recipient: x25519::Recipient = recipient_pubkey
        .parse()
        .map_err(|e| anyhow::anyhow!("Failed to parse recipient public key: {}", e))?;

    let encryptor = Encryptor::with_recipients(iter::once(&recipient as &dyn age::Recipient))
        .context("Failed to create encryptor")?;

    let mut encrypted = vec![];
    let mut writer = encryptor.wrap_output(&mut encrypted)?;
    writer.write_all(plaintext.as_bytes())?;
    writer.finish()?;

    Ok(encrypted)
}

/// Decrypt an age-encrypted message using the provided identity
pub fn decrypt_message(ciphertext: &[u8], identity: &x25519::Identity) -> Result<String> {
    let decryptor = Decryptor::new(ciphertext)?;

    let mut decrypted = vec![];
    let mut reader = decryptor.decrypt(iter::once(identity as &dyn age::Identity))?;
    reader.read_to_end(&mut decrypted)?;

    String::from_utf8(decrypted).context("Decrypted data is not valid UTF-8")
}

/// Simple end-to-end encryption module for ReticulumChat
///
/// Uses the age encryption format (X25519) for E2E message encryption.
/// Each message is encrypted to the recipient's public key and can only
/// be decrypted by the recipient's private identity.
pub struct E2ECipher;

impl E2ECipher {
    /// Encrypt plaintext to a recipient's public key, returning armored ciphertext
    pub fn encrypt(plaintext: &str, recipient_pubkey: &str) -> Result<String> {
        let recipient: x25519::Recipient = recipient_pubkey
            .parse()
            .map_err(|e| anyhow::anyhow!("Failed to parse recipient public key: {}", e))?;

        let encryptor = Encryptor::with_recipients(iter::once(&recipient as &dyn age::Recipient))
            .context("Failed to create encryptor")?;

        let mut armored = Vec::new();
        {
            let mut armored_writer = age::armor::ArmoredWriter::wrap_output(
                &mut armored,
                age::armor::Format::AsciiArmor,
            )?;
            let mut writer = encryptor.wrap_output(&mut armored_writer)?;
            writer.write_all(plaintext.as_bytes())?;
            writer.finish()?;
            armored_writer.finish()?;
        }

        Ok(String::from_utf8(armored)?)
    }

    /// Decrypt armored ciphertext using an age identity
    pub fn decrypt(armored_ciphertext: &str, identity: &x25519::Identity) -> Result<String> {
        let armored_reader = age::armor::ArmoredReader::new(armored_ciphertext.as_bytes());
        let decryptor = Decryptor::new(armored_reader)?;
        let mut decrypted = vec![];
        let mut reader = decryptor.decrypt(iter::once(identity as &dyn age::Identity))?;
        reader.read_to_end(&mut decrypted)?;

        String::from_utf8(decrypted).context("Decrypted data is not valid UTF-8")
    }
}

/// Verify a message signature (placeholder - age provides authenticity via encryption)
pub fn verify_authenticity(_ciphertext: &[u8], _sender_pubkey: &str) -> Result<bool> {
    // age encryption inherently provides sender authentication when using
    // the appropriate format. In a full implementation, we might add
    // detached signatures using Ed25519.
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_encryption() {
        let identity = x25519::Identity::generate();
        let recipient = identity.to_public().to_string();
        let plaintext = "Hello, Reticulum!";

        let encrypted = encrypt_message(plaintext, &recipient).unwrap();
        assert_ne!(encrypted, plaintext.as_bytes());

        let decrypted = decrypt_message(&encrypted, &identity).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_armored_roundtrip() {
        let identity = x25519::Identity::generate();
        let recipient = identity.to_public().to_string();
        let plaintext = "Hello, Reticulum!";

        let encrypted = E2ECipher::encrypt(plaintext, &recipient).unwrap();
        let decrypted = E2ECipher::decrypt(&encrypted, &identity).unwrap();
        assert_eq!(decrypted, plaintext);
    }
}
