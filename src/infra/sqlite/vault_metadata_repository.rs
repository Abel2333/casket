use anyhow::Result;
use sqlx::{Row, SqlitePool};

use crate::{infra::crypto::VaultMetadata, storage::traits::VaultMetadataRepository};

pub struct SqliteVaultMetadataRepository {
    pub pool: SqlitePool,
}

#[async_trait::async_trait]
impl VaultMetadataRepository for SqliteVaultMetadataRepository {
    async fn get(&self) -> Result<Option<VaultMetadata>> {
        let row = sqlx::query(
            r#"
              SELECT salt, key_version, kdf_algorithm, kdf_params_json
              FROM vault_metadata
              WHERE id = 1
            "#,
        )
        .fetch_optional(&self.pool)
        .await?;

        let metadata = row.map(|row| VaultMetadata {
            salt: row.get("salt"),
            key_version: row.get("key_version"),
            kdf_algorithm: row.get("kdf_algorithm"),
            kdf_params_json: row.get("kdf_params_json"),
        });

        Ok(metadata)
    }

    async fn save(&self, metadata: &VaultMetadata) -> Result<()> {
        sqlx::query(
            r#"
              INSERT INTO vault_metadata (
                  id,
                  salt,
                  key_version,
                  kdf_algorithm,
                  kdf_params_json
              )
              VALUES (?, ?, ?, ?, ?)
              ON CONFLICT(id) DO UPDATE SET
                  salt = excluded.salt,
                  key_version = excluded.key_version,
                  kdf_algorithm = excluded.kdf_algorithm,
                  kdf_params_json = excluded.kdf_params_json
            "#,
        )
        .bind(1_i64)
        .bind(&metadata.salt)
        .bind(metadata.key_version)
        .bind(&metadata.kdf_algorithm)
        .bind(&metadata.kdf_params_json)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::infra::crypto::VaultMetadata;
    use crate::infra::sqlite::schema::INIT_SQL;

    #[tokio::test]
    async fn get_while_metadata_is_missing() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        sqlx::query(INIT_SQL).execute(&pool).await?;

        let repo = SqliteVaultMetadataRepository { pool };

        let metadata = repo.get().await?;
        assert!(metadata.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn save_and_get() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        sqlx::query(INIT_SQL).execute(&pool).await?;

        let repo = SqliteVaultMetadataRepository { pool };

        let expected = VaultMetadata {
            salt: vec![1, 2, 3, 4],
            key_version: 1,
            kdf_algorithm: "argon2id".to_string(),
            kdf_params_json: r#"{"memory_cost": 19456, "iterations": 2}"#.to_string(),
        };

        repo.save(&expected).await?;

        let actual = repo.get().await?.expect("metadata should exist");

        assert_eq!(actual.salt, expected.salt);
        assert_eq!(actual.key_version, expected.key_version);
        assert_eq!(actual.kdf_algorithm, expected.kdf_algorithm);
        assert_eq!(actual.kdf_params_json, expected.kdf_params_json);

        Ok(())
    }
}
