use anyhow::{Result, anyhow, bail};
use argon2::{Argon2, Params};
use serde::{Deserialize, Serialize};

use crate::infra::crypto::{KeyDeriver, MasterKey, VaultMetadata};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Argon2ParamsConfig {
    pub memory_cost: u32,
    pub iterations: u32,
    pub parallelism: u32,
}

pub struct Argon2KeyDeriver;

impl KeyDeriver for Argon2KeyDeriver {
    fn derive_key(&self, password: &str, metadata: &VaultMetadata) -> Result<MasterKey> {
        if metadata.kdf_algorithm != "argon2id" {
            bail!("unsupported kdf algorithm: {}", metadata.kdf_algorithm);
        }

        let config: Argon2ParamsConfig = serde_json::from_str(&metadata.kdf_params_json)?;

        let params = Params::new(
            config.memory_cost,
            config.iterations,
            config.parallelism,
            Some(32),
        )
        .map_err(|err| anyhow!(err.to_string()))?;

        let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);

        let mut output = [0u8; 32];
        argon2
            .hash_password_into(password.as_bytes(), &metadata.salt, &mut output)
            .map_err(|err| anyhow!(err.to_string()))?;

        Ok(MasterKey(output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn derives_same_key() -> Result<()> {
        let deriver = Argon2KeyDeriver;
        let metadata = VaultMetadata {
            salt: b"0123456789abcdef".to_vec(),
            key_version: 1,
            kdf_algorithm: "argon2id".to_string(),
            kdf_params_json: r#"{"memory_cost":19456,"iterations":2,"parallelism":1}"#.to_string(),
        };

        let key1 = deriver.derive_key("secret", &metadata)?;
        let key2 = deriver.derive_key("secret", &metadata)?;

        assert_eq!(key1.0, key2.0);
        Ok(())
    }
}
