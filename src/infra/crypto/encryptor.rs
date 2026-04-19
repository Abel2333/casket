use anyhow::anyhow;
use chacha20poly1305::{
    AeadCore, KeyInit, XChaCha20Poly1305, XNonce,
    aead::{Aead, OsRng},
};

use crate::{domain::secret::EncryptedBlob, infra::crypto::Encryptor};

pub struct XChaCha20Poly1305Encryptor;

impl Encryptor for XChaCha20Poly1305Encryptor {
    fn encrypt(
        &self,
        plaintext: &[u8],
        key: &super::MasterKey,
    ) -> anyhow::Result<crate::domain::secret::EncryptedBlob> {
        let cipher = XChaCha20Poly1305::new((&key.0).into());
        let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);

        let ciphertext = cipher
            .encrypt(&nonce, plaintext)
            .map_err(|err| anyhow!(err.to_string()))?;

        Ok(EncryptedBlob {
            ciphertext,
            nonce: nonce.to_vec(),
        })
    }

    fn decrypt(&self, blob: &EncryptedBlob, key: &super::MasterKey) -> anyhow::Result<Vec<u8>> {
        if blob.nonce.len() != 24 {
            return Err(anyhow!(
                "nonce length should equal to 24, not {}",
                blob.nonce.len()
            ));
        }

        let cipher = XChaCha20Poly1305::new((&key.0).into());

        let nonce = XNonce::from_slice(&blob.nonce);

        let plaintext = cipher
            .decrypt(nonce, blob.ciphertext.as_ref())
            .map_err(|err| anyhow!(err.to_string()))?;

        Ok(plaintext)
    }
}

#[cfg(test)]
mod tests {
    use crate::infra::crypto::MasterKey;
    use anyhow::Result;

    use super::*;

    #[test]
    fn encrypt_and_decrypt_correct() -> Result<()> {
        let encryptor = XChaCha20Poly1305Encryptor;
        let key = MasterKey([7u8; 32]);

        let blob = encryptor.encrypt(b"hello", &key)?;
        let plaintext = encryptor.decrypt(&blob, &key)?;

        assert_eq!(plaintext, b"hello");
        Ok(())
    }

    #[test]
    fn decrypt_fails_with_wrong_key() -> Result<()> {
        let encryptor = XChaCha20Poly1305Encryptor;
        let key1 = MasterKey([7u8; 32]);
        let key2 = MasterKey([8u8; 32]);

        let blob = encryptor.encrypt(b"hello", &key1)?;
        let result = encryptor.decrypt(&blob, &key2);

        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn encrypt_uses_fresh_nonce() -> Result<()> {
        let encryptor = XChaCha20Poly1305Encryptor;
        let key = MasterKey([7u8; 32]);

        let blob1 = encryptor.encrypt(b"hello", &key)?;
        let blob2 = encryptor.encrypt(b"hello", &key)?;

        assert_ne!(blob1.nonce, blob2.nonce);
        Ok(())
    }
}
