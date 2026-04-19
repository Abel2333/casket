pub mod encryptor;
pub mod key_deriver;
use crate::domain::secret::EncryptedBlob;
pub use encryptor::XChaCha20Poly1305Encryptor;
pub struct MasterKey(pub [u8; 32]);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultMetadata {
    pub salt: Vec<u8>,
    pub key_version: i32,
    pub kdf_algorithm: String,
    pub kdf_params_json: String,
}

pub trait VaultMetadataFactory {
    fn new_metadata(&self) -> anyhow::Result<VaultMetadata>;
}

pub trait KeyDeriver {
    fn derive_key(&self, password: &str, metadata: &VaultMetadata) -> anyhow::Result<MasterKey>;
}

pub trait Encryptor {
    fn encrypt(&self, plaintext: &[u8], key: &MasterKey) -> anyhow::Result<EncryptedBlob>;

    fn decrypt(&self, blob: &EncryptedBlob, key: &MasterKey) -> anyhow::Result<Vec<u8>>;
}
