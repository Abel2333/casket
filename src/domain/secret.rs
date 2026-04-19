use uuid::Uuid;

use crate::domain::entry::EntryId;

pub type SecretFieldId = Uuid;

#[derive(Debug, Clone)]
pub struct SecretField {
    pub id: SecretFieldId,
    pub entry_id: EntryId,
    pub name: String,
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
    pub key_version: i32,
}

#[derive(Debug, Clone)]
pub struct SecretDraftField {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct EncryptedBlob {
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecryptedSecretField {
    pub name: String,
    pub value: String,
}
