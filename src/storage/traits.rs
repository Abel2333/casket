use crate::{
    domain::{
        entry::{Entry, EntryFilter, EntryId},
        secret::SecretField,
        tag::Tag,
    },
    infra::crypto::VaultMetadata,
};
use anyhow::Result;

#[async_trait::async_trait]
pub trait EntryRepository {
    async fn create(&self, entry: &Entry) -> Result<()>;
    async fn update(&self, entry: &Entry) -> Result<()>;
    async fn get(&self, id: EntryId) -> Result<Option<Entry>>;
    async fn list(&self, filter: &EntryFilter) -> Result<Vec<Entry>>;
    async fn delete(&self, id: EntryId) -> Result<()>;
}

#[async_trait::async_trait]
pub trait SecretRepository {
    async fn replace_for_entry(&self, entry_id: EntryId, fields: &[SecretField]) -> Result<()>;
    async fn list_for_entry(&self, entry_id: EntryId) -> Result<Vec<SecretField>>;
}

#[async_trait::async_trait]
pub trait TagRepository {
    async fn replace_for_entry(&self, entry_id: EntryId, tag_names: &[String]) -> Result<()>;
    async fn list_for_entry(&self, entry_id: EntryId) -> Result<Vec<Tag>>;
    async fn list_all(&self) -> Result<Vec<Tag>>;
}

#[async_trait::async_trait]
pub trait VaultMetadataRepository {
    async fn get(&self) -> Result<Option<VaultMetadata>>;
    async fn save(&self, metadata: &VaultMetadata) -> Result<()>;
}
