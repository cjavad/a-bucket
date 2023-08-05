use std::{
    path::{self, PathBuf},
    time::SystemTime,
};

use a_http_parser::http::MimeType;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    authentication::{AuthContext, AuthLevel},
    metadata::Metadata,
    storable::{StorableBase, StorableBlob, StorableJson}, TMP_PATH,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Object {
    key: String,
    pub metadata: Metadata,
    data: Option<Vec<u8>>,
}

impl StorableBase for Object {
    fn base_dir() -> PathBuf {
        format!("{}/storage", TMP_PATH).into()
    }

    fn id(&self) -> &str {
        self.key.as_str()
    }
}

impl StorableBlob for Object {
    fn get_data(&self) -> Vec<u8> {
        if self.data.is_none() {
            return Vec::new();
        }

        self.data.clone().unwrap()
    }
}

pub struct Storage {
    auth_context: AuthContext,
}

impl Storage {
    pub fn new(auth_context: AuthContext) -> Self {
        Self { auth_context }
    }

    pub async fn get_object(&self, key: &str, read_data: bool) -> Option<Object> {
        let metadata = match Metadata::load(key).await {
            Ok(metadata) => metadata,
            Err(_) => return None,
        };

        if !self.is_object_readable(&metadata).await {
            return None;
        }

        // Use get_object to fetch ownership of the metadata and do
        // the access check. If we don't need to read the data, we can
        // return early
        if !read_data {
            return Some(Object {
                key: key.to_string(),
                metadata,
                data: None,
            });
        }

        // We own the metadata now, so now we can load the data
        let data = match Object::load(key).await {
            Ok(data) => Some(data),
            Err(_) => return None,
        };

        Some(Object {
            key: key.to_string(),
            metadata,
            data,
        })
    }

    pub async fn put_object(&self, key: &str, data: &[u8], mime_type: MimeType, readable_by: AuthLevel) -> bool {
        if let Some(object) = self.get_object(key, false).await {
            if !self.is_object_writable(&object.metadata).await {
                return false;
            }
        };

        let mut hasher = Sha256::new();
        hasher.update(data);

        let metadata = Metadata {
            name: key
                .split_terminator(path::MAIN_SEPARATOR)
                .last()
                .unwrap()
                .to_string(),
            key: key.to_string(),
            size: data.len() as u64,
            last_modified: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            etag: hex::encode(hasher.finalize()),
            mime_type: mime_type.to_str().to_string(),
            owner_id: self.auth_context.access_key.clone(),
            readable_by
        };

        match &metadata.save().await {
            Ok(_) => {
                let object = Object {
                    key: key.to_string(),
                    metadata,
                    data: Some(data.to_vec()),
                };


                match object.save().await {
                    Ok(_) => true,
                    Err(_) => {
                        object.metadata.delete().await.unwrap_or_else(|_| {});                    

                        false
                    },
                }
            }
            Err(_) => return false,
        }
    }

    pub async fn delete_object(&self, key: &str) -> bool {
        if let Some(object) = self.get_object(key, false).await {
            if !self.is_object_writable(&object.metadata).await {
                return false;
            }

            match object.metadata.delete().await {
                Ok(_) => {}
                Err(_) => return false,
            }

            match object.delete().await {
                Ok(_) => true,
                Err(_) => false,
            }
        } else {
            false
        }
    }

    pub async fn list_objects(&self) -> Vec<Metadata> {
        let mut objects = Vec::new();

        if let Ok(mut list) = Metadata::list().await {
            while let Some(metadata) = list.next().await.unwrap() {
                if self.is_object_readable(&metadata).await {
                    objects.push(metadata);
                }
            }

            objects
        } else {
            Vec::new()
        }
    }

    pub async fn is_object_readable(&self, metadata: &Metadata) -> bool {
        if metadata.readable_by == AuthLevel::Public {
            return true;
        }

        match AuthContext::load(&self.auth_context.access_key).await {
            Ok(context) => {
                match AuthContext::load(&metadata.owner_id).await {
                    Ok(owner_context) => {
                        // Admins cannot read other admins' objects        
                        if self.auth_context.access_level == AuthLevel::Admin && owner_context.access_level != AuthLevel::Admin {
                            return true;
                        }

                        if metadata.readable_by == AuthLevel::Owner && self.auth_context == owner_context {
                            return true;
                        }
                    },
                    Err(_) => return context.access_level == AuthLevel::Admin,
                }
  
                if metadata.readable_by <= self.auth_context.access_level {
                    return true;
                }
        
            }
            Err(_) => return false,
        }
      

        return false;

    }

    pub async fn is_object_writable(&self, metadata: &Metadata) -> bool {
        match AuthContext::load(&self.auth_context.access_key).await {
            Ok(context) => {
                match AuthContext::load(&metadata.owner_id).await {
                    Ok(owner_context) => {
                        // Admins cannot write other admins' objects
                        if self.auth_context.access_level == AuthLevel::Admin {
                            if context.access_level == AuthLevel::Admin {
                                return context.access_key == self.auth_context.access_key;
                            }

                            return true;
                        }
                
                        if self.auth_context == owner_context {
                            return true;
                        }
                    },
                    Err(_) => return context.access_level == AuthLevel::Admin,
                }
            }
            Err(_) => return false,
        }

        return false;
    }
}
