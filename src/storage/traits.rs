use crate::{
    domain::{
        entry::{Entry, EntryFilter, EntryId},
        secret::SecretField,
        tag::Tag,
    },
    infra::crypto::VaultMetadata,
};
use anyhow::Result;

pub trait EntryRepository {
    fn create(&self, entry: &Entry) -> Result<()>;
    fn update(&self, entry: &Entry) -> Result<()>;
    fn get(&self, id: EntryId) -> Result<Option<Entry>>;
    fn list(&self, filter: &EntryFilter) -> Result<Vec<Entry>>;
    fn delete(&self, id: EntryId) -> Result<()>;
}

pub trait SecretRepository {
    fn replace_for_entry(&self, entry_id: EntryId, fields: &[SecretField]) -> Result<()>;
    fn list_for_entry(&self, entry_id: EntryId) -> Result<Vec<SecretField>>;
}

pub trait TagRepository {
    fn replace_for_entry(&self, entry_id: EntryId, tag_names: &[String]) -> Result<()>;
    fn list_for_entry(&self, entry_id: EntryId) -> Result<Vec<Tag>>;
    fn list_all(&self) -> Result<Vec<Tag>>;
}

#[async_trait::async_trait]
pub trait VaultMetadataRepository {
    async fn get(&self) -> Result<Option<VaultMetadata>>;
    async fn save(&self, metadata: &VaultMetadata) -> Result<()>;
}
