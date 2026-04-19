use anyhow::{Result, anyhow};
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
    async fn save_secret(
        &self,
        entry_id: EntryId,
        secret_fields: Vec<SecretDraftField>,
        key: Option<&MasterKey>,
    ) -> Result<()> {
        if secret_fields.is_empty() {
            self.secrets.replace_for_entry(entry_id, &[]).await?;
            return Ok(());
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

        self.secrets
            .replace_for_entry(entry_id, &encrypted_fields)
            .await
    }

    pub async fn create_from_draft(
        &self,
        draft: EntryDraft,
        key: Option<&MasterKey>,
    ) -> Result<EntryId> {
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

        self.entries.create(&entry).await?;
        self.tags.replace_for_entry(entry_id, &draft.tags).await?;

        self.save_secret(entry_id, draft.secret_fields, key).await?;

        Ok(entry_id)
    }

    pub async fn update_from_draft(
        &self,
        draft: EntryDraft,
        key: Option<&MasterKey>,
    ) -> Result<()> {
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
            .get(entry_id)
            .await?
            .ok_or_else(|| anyhow!("Entry not found"))?;

        entry.kind = kind;
        entry.title = title;
        entry.body = body;
        entry.updated_at = Utc::now();

        self.entries.update(&entry).await?;
        self.tags.replace_for_entry(entry_id, &tags).await?;

        self.save_secret(entry_id, secret_fields, key).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use anyhow::Result;

    use super::*;
    use crate::domain::{
        entry::{EntryFilter, EntryKind},
        secret::EncryptedBlob,
        tag::Tag,
    };

    struct FakeEntryRepo {
        created: Mutex<Vec<Entry>>,
        stored: Mutex<Option<Entry>>,
    }

    #[async_trait::async_trait]
    impl EntryRepository for FakeEntryRepo {
        async fn create(&self, entry: &Entry) -> Result<()> {
            self.created.lock().unwrap().push(entry.clone());
            self.stored.lock().unwrap().replace(entry.clone());
            Ok(())
        }

        async fn update(&self, entry: &Entry) -> Result<()> {
            self.stored.lock().unwrap().replace(entry.clone());
            Ok(())
        }

        async fn get(&self, _id: EntryId) -> Result<Option<Entry>> {
            Ok(self.stored.lock().unwrap().clone())
        }

        async fn list(&self, _filter: &EntryFilter) -> Result<Vec<Entry>> {
            Ok(self.created.lock().unwrap().clone())
        }

        async fn delete(&self, _id: EntryId) -> Result<()> {
            Ok(())
        }
    }

    struct FakeSecretRepo {
        saved: Mutex<Vec<SecretField>>,
    }

    #[async_trait::async_trait]
    impl SecretRepository for FakeSecretRepo {
        async fn replace_for_entry(
            &self,
            _entry_id: EntryId,
            fields: &[SecretField],
        ) -> Result<()> {
            let mut saved = self.saved.lock().unwrap();
            saved.clear();
            saved.extend_from_slice(fields);
            Ok(())
        }

        async fn list_for_entry(&self, _entry_id: EntryId) -> Result<Vec<SecretField>> {
            Ok(self.saved.lock().unwrap().clone())
        }
    }

    struct FakeTagRepo {
        tags: Mutex<Vec<String>>,
    }

    #[async_trait::async_trait]
    impl TagRepository for FakeTagRepo {
        async fn replace_for_entry(&self, _entry_id: EntryId, tag_names: &[String]) -> Result<()> {
            let mut tags = self.tags.lock().unwrap();
            tags.clear();
            tags.extend(tag_names.iter().cloned());
            Ok(())
        }

        async fn list_for_entry(&self, _entry_id: EntryId) -> Result<Vec<Tag>> {
            Ok(vec![])
        }

        async fn list_all(&self) -> Result<Vec<Tag>> {
            Ok(vec![])
        }
    }

    struct FakeEncryptor;

    impl Encryptor for FakeEncryptor {
        fn encrypt(&self, plaintext: &[u8], _key: &MasterKey) -> Result<EncryptedBlob> {
            let mut ciphertext = b"enc:".to_vec();
            ciphertext.extend_from_slice(plaintext);

            Ok(EncryptedBlob {
                ciphertext,
                nonce: vec![1, 2, 3, 4],
            })
        }

        fn decrypt(&self, blob: &EncryptedBlob, _key: &MasterKey) -> Result<Vec<u8>> {
            Ok(blob.ciphertext.clone())
        }
    }

    fn make_service() -> EntryService<FakeEntryRepo, FakeSecretRepo, FakeTagRepo, FakeEncryptor> {
        EntryService {
            entries: FakeEntryRepo {
                created: Mutex::new(vec![]),
                stored: Mutex::new(None),
            },
            secrets: FakeSecretRepo {
                saved: Mutex::new(vec![]),
            },
            tags: FakeTagRepo {
                tags: Mutex::new(vec![]),
            },
            encryptor: FakeEncryptor,
        }
    }

    fn sample_draft(secret_fields: Vec<SecretDraftField>) -> EntryDraft {
        EntryDraft {
            id: None,
            kind: EntryKind::Note,
            title: "Test".to_string(),
            body: "Body".to_string(),
            tags: vec!["alpha".to_string(), "beta".to_string()],
            secret_fields,
        }
    }

    #[tokio::test]
    async fn create_empty_secret() -> Result<()> {
        let service = make_service();
        let draft = sample_draft(vec![]);

        let _entry_id = service.create_from_draft(draft, None).await?;

        let saved = service.secrets.saved.lock().unwrap().clone();
        assert!(saved.is_empty());

        let tags = service.tags.tags.lock().unwrap().clone();
        assert_eq!(tags, vec!["alpha".to_string(), "beta".to_string()]);

        Ok(())
    }

    #[tokio::test]
    async fn create_with_secret() -> Result<()> {
        let service = make_service();
        let draft = sample_draft(vec![SecretDraftField {
            name: "password".to_string(),
            value: "super-secret".to_string(),
        }]);

        let key = MasterKey([7u8; 32]);

        let entry_id = service.create_from_draft(draft, Some(&key)).await?;

        let saved = service.secrets.saved.lock().unwrap().clone();
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].entry_id, entry_id);
        assert_eq!(saved[0].name, "password");
        assert_eq!(saved[0].ciphertext, b"enc:super-secret".to_vec());
        assert_eq!(saved[0].nonce, vec![1, 2, 3, 4]);

        Ok(())
    }

    #[tokio::test]
    async fn create_requires_master_key() {
        let service = make_service();
        let draft = sample_draft(vec![SecretDraftField {
            name: "password".to_string(),
            value: "super-secret".to_string(),
        }]);

        let result = service.create_from_draft(draft, None).await;

        assert!(result.is_err());
    }
}
