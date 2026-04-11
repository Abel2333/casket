use anyhow::{Result, anyhow, bail};
use chrono::Utc;
use uuid::Uuid;

use crate::{
    domain::{
        entry::{Entry, EntryDraft, EntryId},
        secret::{SecretDraftField, SecretField},
    },
    infra::crypto::{Encryptor, MasterKey},
    storage::traits::{EntryRepository, SecretRepository, TagRepository},
};

pub struct EntryService<ER, SR, TR, ENC> {
    pub entries: ER,
    pub secrets: SR,
    pub tags: TR,
    pub encryptor: ENC,
}

impl<ER, SR, TR, ENC> EntryService<ER, SR, TR, ENC>
where
    ER: EntryRepository,
    SR: SecretRepository,
    TR: TagRepository,
    ENC: Encryptor,
{
    fn save_secret(
        &self,
        entry_id: EntryId,
        secret_fields: Vec<SecretDraftField>,
        key: Option<&MasterKey>,
    ) -> Result<()> {
        if secret_fields.is_empty() {
            self.secrets.replace_for_entry(entry_id, &[])?;
        }

        let key = key.ok_or_else(|| anyhow!("master key required for secret fields"))?;
        let encrypted_fields = secret_fields
            .into_iter()
            .map(|field| {
                let blob = self.encryptor.encrypt(field.value.as_bytes(), key)?;

                Ok(SecretField {
                    id: Uuid::now_v7(),
                    entry_id,
                    name: field.name,
                    ciphertext: blob.ciphertext,
                    nonce: blob.nonce,
                    key_version: 1,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        self.secrets.replace_for_entry(entry_id, &encrypted_fields)
    }

    pub fn create_from_draft(&self, draft: EntryDraft, key: Option<&MasterKey>) -> Result<EntryId> {
        let entry_id = draft.id.unwrap_or_else(Uuid::now_v7);

        let entry = Entry {
            id: entry_id,
            kind: draft.kind,
            title: draft.title,
            body: draft.body,
            is_favorite: false,
            is_archived: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        self.entries.create(&entry)?;
        self.tags.replace_for_entry(entry_id, &draft.tags)?;

        self.save_secret(entry_id, draft.secret_fields, key)?;

        Ok(entry_id)
    }

    pub fn update_from_draft(&self, draft: EntryDraft, key: Option<&MasterKey>) -> Result<()> {
        let EntryDraft {
            id,
            kind,
            title,
            body,
            tags,
            secret_fields,
        } = draft;

        let entry_id =
            id.ok_or_else(|| anyhow!("Could not update an entry from a draft without an id"))?;

        let mut entry = self
            .entries
            .get(entry_id)?
            .ok_or_else(|| anyhow!("Entry not found"))?;

        entry.kind = kind;
        entry.title = title;
        entry.body = body;
        entry.updated_at = Utc::now();

        self.entries.update(&entry)?;
        self.tags.replace_for_entry(entry_id, &tags)?;

        self.save_secret(entry_id, secret_fields, key)?;

        Ok(())
    }
}
