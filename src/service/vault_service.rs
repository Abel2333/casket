use crate::{
    infra::crypto::{KeyDeriver, MasterKey, VaultMetadata, VaultMetadataFactory},
    storage::traits::VaultMetadataRepository,
};
use anyhow::{Result, anyhow, bail};

pub struct VaultService<VR, KD, VF> {
    pub metadata_repo: VR,
    pub key_deriver: KD,
    pub metadata_factory: VF,
}

impl<VR, KD, VF> VaultService<VR, KD, VF>
where
    VR: VaultMetadataRepository,
    KD: KeyDeriver,
    VF: VaultMetadataFactory,
{
    pub async fn is_initialized(&self) -> Result<bool> {
        Ok(self.metadata_repo.get().await?.is_some())
    }

    pub async fn initialize(&self, password: &str) -> Result<MasterKey> {
        if self.metadata_repo.get().await?.is_some() {
            bail!("vault already initialized");
        }

        let metadata = self.metadata_factory.new_metadata()?;
        let key = self.key_deriver.derive_key(password, &metadata)?;

        self.metadata_repo.save(&metadata).await?;

        Ok(key)
    }

    pub async fn unlock(&self, password: &str) -> Result<MasterKey> {
        let metadata = self
            .metadata_repo
            .get()
            .await?
            .ok_or_else(|| anyhow!("vault not initialized"))?;

        let key = self.key_deriver.derive_key(password, &metadata)?;

        Ok(key)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    struct FakeMetadataRepo {
        metadata: Mutex<Option<VaultMetadata>>,
    }

    #[async_trait::async_trait]
    impl VaultMetadataRepository for FakeMetadataRepo {
        async fn get(&self) -> Result<Option<VaultMetadata>> {
            Ok(self.metadata.lock().unwrap().clone())
        }

        async fn save(&self, metadata: &VaultMetadata) -> Result<()> {
            *self.metadata.lock().unwrap() = Some(metadata.clone());
            Ok(())
        }
    }

    struct FakeKeyDeriver;

    struct FakeMetadataFactory;

    impl KeyDeriver for FakeKeyDeriver {
        fn derive_key(
            &self,
            password: &str,
            metadata: &VaultMetadata,
        ) -> anyhow::Result<MasterKey> {
            let mut key = [0_u8; 32];

            for (i, b) in password.as_bytes().iter().take(16).enumerate() {
                key[i] = *b;
            }

            for (i, b) in metadata.salt.iter().take(16).enumerate() {
                key[16 + i] = *b;
            }

            Ok(MasterKey(key))
        }
    }

    impl VaultMetadataFactory for FakeMetadataFactory {
        fn new_metadata(&self) -> anyhow::Result<VaultMetadata> {
            Ok(VaultMetadata {
                salt: vec![1, 2, 3, 4],
                key_version: 1,
                kdf_algorithm: "argon2id".to_string(),
                kdf_params_json: r#"{"memory_cost":19456,"iterations":2,"parallelism":1}"#
                    .to_string(),
            })
        }
    }

    fn make_service(
        metadata: Option<VaultMetadata>,
    ) -> VaultService<FakeMetadataRepo, FakeKeyDeriver, FakeMetadataFactory> {
        VaultService {
            metadata_repo: FakeMetadataRepo {
                metadata: Mutex::new(metadata),
            },
            key_deriver: FakeKeyDeriver,
            metadata_factory: FakeMetadataFactory,
        }
    }

    #[tokio::test]
    async fn uninitialized() -> Result<()> {
        let service = make_service(None);

        let initialized = service.is_initialized().await?;

        assert!(!initialized);
        Ok(())
    }

    #[tokio::test]
    async fn initialized() -> Result<()> {
        let service = make_service(None);

        let key = service.initialize("secret").await?;

        assert_eq!(key.0[0], b's');

        let saved = service.metadata_repo.get().await?;
        assert!(saved.is_some());

        Ok(())
    }

    #[tokio::test]
    async fn unlock_while_uninitialized() {
        let service = make_service(None);

        let result = service.unlock("secret").await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn unlock_with_same_key() -> Result<()> {
        let service = make_service(None);

        let initialized_key = service.initialize("secret").await?;
        let unlocked_key = service.unlock("secret").await?;

        assert_eq!(initialized_key.0, unlocked_key.0);
        Ok(())
    }
}
