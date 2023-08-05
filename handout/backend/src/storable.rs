use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::fs;
use tokio::fs::File;
use tokio::io::{AsyncRead, AsyncReadExt, ReadBuf};
use tokio_stream::Stream;

use crate::TMP_PATH;
#[async_trait]
pub trait StorableBase {
    fn base_dir() -> PathBuf;
    fn id(&self) -> &str;

    fn ensure_base_dir_exists() -> std::io::Result<()> {
        if !Self::base_dir().exists() {
            let _ = std::fs::create_dir_all(Self::base_dir());
        }
        Ok(())
    }

    async fn canonicalize_path(path: PathBuf) -> std::io::Result<PathBuf> {
        let path = fs::canonicalize(path).await?;
        if !path.starts_with(TMP_PATH) {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Invalid path",
            ))
        } else {
            Ok(path)
        }
    }
}

pub struct StorableIterator<S: StorableJson> {
    read_dir: fs::ReadDir,
    _marker: std::marker::PhantomData<S>,
}

impl<S: StorableJson> StorableIterator<S> {
    pub async fn next(&mut self) -> std::io::Result<Option<S>> {
        if let Some(entry) = self.read_dir.next_entry().await? {
            let path = entry.path();
            let contents = fs::read_to_string(&path).await?;
            let result: S = serde_json::from_str(&contents)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }
}

#[async_trait]
pub trait StorableJson: StorableBase + Serialize + DeserializeOwned {
    fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    fn get_path(&self) -> PathBuf {
        Self::base_dir().join(format!(
            "{}.json",
            hex::encode(Sha256::digest(self.id().as_bytes()))
        ))
    }

    async fn delete(&self) -> std::io::Result<()> {
        let _ = Self::ensure_base_dir_exists()?;
        let path = Self::canonicalize_path(self.get_path()).await?;
        fs::remove_file(path).await
    }

    async fn load(id: &str) -> Result<Self, String> {
        let _ = Self::ensure_base_dir_exists()
            .map_err(|e| format!("Failed to create base dir: {}", e))?;

        let path = Self::base_dir().join(format!(
            "{}.json",
            hex::encode(Sha256::digest(id.as_bytes()))
        ));

        let path = Self::canonicalize_path(path)
            .await
            .map_err(|e| format!("Invalid path: {}", e))?;
        let mut file = fs::File::open(&path)
            .await
            .map_err(|e| format!("Failed to open file: {}", e))?;

        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .await
            .map_err(|e| format!("Failed to read file: {}", e))?;

        // Loading the instance from the file
        serde_json::from_str(&contents).map_err(|e| format!("Failed to deserialize: {}", e))
    }

    // Saves the instance to the appropriate directory
    async fn save(&self) -> std::io::Result<()> {
        // Ensure base directory exists
        let _ = Self::ensure_base_dir_exists()?;
        let json = self.to_json().expect("Failed to serialize to JSON");
        let path = Self::base_dir().join(format!(
            "{}.json",
            hex::encode(Sha256::digest(self.id().as_bytes()))
        ));

        // Normalize path to ensure it's in TMP_PATH
        let parent = path.parent().unwrap();
        let parent = Self::canonicalize_path(parent.to_path_buf())
            .await
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "Invalid path"))?;

        // Ensure path root is in TMP_PATH
        if !parent.starts_with(TMP_PATH) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Invalid path",
            ));
        }

        fs::write(path, json).await
    }

    async fn list() -> std::io::Result<StorableIterator<Self>> {
        let read_dir = fs::read_dir(Self::base_dir()).await?;
        Ok(StorableIterator {
            read_dir,
            _marker: std::marker::PhantomData,
        })
    }
}

pub struct FileIterator<R: AsyncRead + Unpin> {
    reader: R,
    buffer: Vec<u8>,
}

impl<R: AsyncRead + Unpin> FileIterator<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            buffer: Vec::new(),
        }
    }
}

impl<R: AsyncRead + Unpin> Stream for FileIterator<R> {
    type Item = std::io::Result<Vec<u8>>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Split the mutable borrow of self into two parts
        let self_mut = self.as_mut().get_mut();
        let reader = Pin::new(&mut self_mut.reader);
        let buffer = &mut self_mut.buffer;

        // Ensure the buffer has some capacity
        buffer.resize(4096, 0);

        // Create a ReadBuf based on the buffer
        let mut read_buf = ReadBuf::new(buffer);

        match reader.poll_read(cx, &mut read_buf) {
            Poll::Ready(Ok(_)) => {
                let len = read_buf.filled().len();
                if len == 0 {
                    Poll::Ready(None) // End of file
                } else {
                    // Resize buffer to the number of bytes that were read
                    buffer.truncate(len);
                    // Return the buffer
                    Poll::Ready(Some(Ok(buffer.clone())))
                }
            }
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(e)) => Poll::Ready(Some(Err(e))),
        }
    }
}


#[async_trait]
pub trait StorableBlob: StorableBase {
    fn get_data(&self) -> Vec<u8>;

    fn get_path(&self) -> PathBuf {
        Self::base_dir().join(self.id())
    }

    async fn delete(&self) -> std::io::Result<()> {
        let _ = Self::ensure_base_dir_exists()?;
        let path = Self::canonicalize_path(self.get_path()).await?;
        tokio::fs::remove_file(path).await
    }

    async fn load(id: &str) -> std::io::Result<Vec<u8>> {
        let path = Self::base_dir().join(id);
        let path = Self::canonicalize_path(path).await?;
        tokio::fs::read(path).await
    }

    async fn stream_file(&self) -> Result<FileIterator<tokio::fs::File>, ()> {
        let _ = Self::ensure_base_dir_exists().map_err(|_| ())?;
        let path = Self::base_dir().join(self.id());
        let path = Self::canonicalize_path(path).await.map_err(|_| ())?;
        let file = File::open(path).await.map_err(|_| ())?;
        Ok(FileIterator::new(file))
    }

    async fn get_file_size(&self) -> Result<u64, ()> {
        let _ = Self::ensure_base_dir_exists().map_err(|_| ())?;
        let path = Self::base_dir().join(self.id());
        let path = Self::canonicalize_path(path).await.map_err(|_| ())?;
        let metadata: std::fs::Metadata = tokio::fs::metadata(path).await.map_err(|_| ())?;
        Ok(metadata.len())
    }

    async fn save(&self) -> std::io::Result<()> {
        let _ = Self::ensure_base_dir_exists()?;
        let path = self.get_path();
        let parent = path.parent().unwrap();

        if !parent.exists() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let _ = Self::canonicalize_path(parent.to_path_buf()).await?;
        tokio::fs::write(path, self.get_data()).await
    }
}
