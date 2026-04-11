use crate::{
    infra::crypto::{Argon2ParamsConfig, KeyDeriver, MasterKey, VaultMetadata},
    storage::traits::VaultMetadataRepository,
};
use anyhow::{Result, anyhow, bail};
use rand::RngExt;

pub struct VaultService<VR, KD> {
    pub metadata_repo: VR,
    pub key_deriver: KD,
}

impl<VR, KD> VaultService<VR, KD>
where
    VR: VaultMetadataRepository,
    KD: KeyDeriver,
{
    pub async fn is_initialized(&self) -> Result<bool> {
        Ok(self.metadata_repo.get().await?.is_some())
    }

    pub async fn initialize(&self, password: &str) -> Result<MasterKey> {
        if self.metadata_repo.get().await?.is_some() {
            bail!("vault already initialized");
        }

        let metadata = VaultMetadata {
            salt: generate_salt(),
            key_version: 1,
            kdf_algorithm: "argon2id".to_string(),
            kdf_params_json: serde_json::to_string(&Argon2ParamsConfig {
                memory_cost: 19_456,
                iterations: 2,
                parallelism: 1,
            })?,
        };

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

fn generate_salt() -> Vec<u8> {
    let mut salt = vec![0u8; 16];

    rand::rng().fill(&mut salt);
    salt
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

    fn make_service(
        metadata: Option<VaultMetadata>,
    ) -> VaultService<FakeMetadataRepo, FakeKeyDeriver> {
        VaultService {
            metadata_repo: FakeMetadataRepo {
                metadata: Mutex::new(metadata),
            },
            key_deriver: FakeKeyDeriver,
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
