use std::{collections::BTreeMap, path::PathBuf, time::SystemTime};

use base64::{engine::general_purpose, Engine as _};
use hmac::{Hmac, Mac};
use jwt::{Header, SignWithKey, Token, VerifyWithKey};
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use sha2::Sha384;

use crate::{storable::{StorableBase, StorableJson}, TMP_PATH};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum AuthLevel {
    Public = 0,
    Read = 1,
    ReadWrite = 2,
    Owner = 3,
    Admin = 37,
}

impl AuthLevel {
    pub fn to_string(&self) -> String {
        match self {
            AuthLevel::Public => "Public",
            AuthLevel::Read => "Read",
            AuthLevel::ReadWrite => "ReadWrite",
            AuthLevel::Owner => "Owner",
            AuthLevel::Admin => "Admin",
        }
        .into()
    }

    pub fn from_string(level: &str) -> Self {
        match level {
            "Public" => AuthLevel::Public,
            "Read" => AuthLevel::Read,
            "ReadWrite" => AuthLevel::ReadWrite,
            "Owner" => AuthLevel::Owner,
            "Admin" => AuthLevel::Admin,
            _ => AuthLevel::Public
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthContext {
    pub access_key: String,
    pub access_level: AuthLevel,
    pub last_used: u64,
    secret_key: String,
}

impl AuthContext {
    pub fn random() -> Self {
        let access_key: String = general_purpose::STANDARD.encode(
            rand::thread_rng()
                .sample_iter(Alphanumeric)
                .take(20)
                .collect::<Vec<u8>>(),
        );

        let secret_key: String = general_purpose::STANDARD.encode(
            rand::thread_rng()
                .sample_iter(Alphanumeric)
                .take(40) // Adjust the length of the generated secret key as needed
                .collect::<Vec<u8>>(),
        );

        AuthContext {
            access_key,
            access_level: AuthLevel::Public,
            last_used: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            secret_key,
        }
    }

    pub fn update_last_used(&mut self) {
        self.last_used = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }

    pub fn as_jwt(&self) -> String {
        let Ok(secret_key) = std::env::var("JWT_SECRET") else {
            panic!("JWT_SECRET not set");
        };

        let key: Hmac<Sha384> = Hmac::new_from_slice(secret_key.as_bytes()).unwrap();
        let mut claims = BTreeMap::new();
        claims.insert("id", self.id());
        let access_level = self.access_level.clone().to_string();
        claims.insert("access_level", &access_level);
        claims.sign_with_key(&key).unwrap()
    }

    pub fn id_from_jwt(token_str: &String) -> (Option<String>, Option<AuthLevel>) {
        let Ok(secret_key) = std::env::var("JWT_SECRET") else {
            panic!("JWT_SECRET not set");
        };

        let key: Hmac<Sha384> = Hmac::new_from_slice(secret_key.as_bytes()).unwrap();
        let token: Token<Header, BTreeMap<String, String>, _> =
            match token_str.verify_with_key(&key) {
                Ok(token) => token,
                Err(_) => return (None, None),
            };

        let claims = token.claims();

        (
            claims.get("id").map(|id| id.to_string()),
            claims
                .get("access_level")
                .map(|access_level| AuthLevel::from_string(access_level)),
        )
    }
}

impl StorableBase for AuthContext {
    fn base_dir() -> PathBuf {
        format!("{}/auth", TMP_PATH).into()
    }

    fn id(&self) -> &str {
        &self.access_key.as_str()
    }
}

impl StorableJson for AuthContext {}

// Write tests to see if == works

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_context_jwt() {
        let auth_context = AuthContext {
            access_key: "test".into(),
            secret_key: "None".into(),
            access_level: AuthLevel::Admin,
            last_used: 0,
        };

        let auth_context_2 = AuthContext {
            access_key: "test".into(),
            secret_key: "None".into(),
            access_level: AuthLevel::Admin,
            last_used: 0,
        };

        assert_eq!(auth_context, auth_context_2);
    }

    #[test]
    fn test_auth_level_comparison() {
        assert!(AuthLevel::Admin > AuthLevel::ReadWrite);
        assert!(AuthLevel::ReadWrite > AuthLevel::Read);
        assert!(AuthLevel::Read > AuthLevel::Public);
    }
}
