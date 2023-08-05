use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{storable::{StorableBase, StorableJson}, authentication::AuthLevel, TMP_PATH};

#[derive(Debug, Serialize, Deserialize)]
pub struct Metadata {
    pub name: String,
    pub key: String,
    pub size: u64,
    pub last_modified: u64,
    pub etag: String,
    pub mime_type: String,
    pub owner_id: String,
    pub readable_by: AuthLevel,
}

impl StorableBase for Metadata {
    fn base_dir() -> PathBuf {
        format!("{}/metadata", TMP_PATH).into()
    }

    fn id(&self) -> &str {
        &self.key
    }
}

impl StorableJson for Metadata {}
