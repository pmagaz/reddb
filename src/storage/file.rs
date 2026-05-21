use async_trait::async_trait;
use core::fmt::Debug;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use super::Storage;
use crate::document::Document;
use crate::error::{RedDbError, Result};
use crate::serializer::{Serializer, Serializers};
use crate::status::Status;
use crate::RedDbHM;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncBufReadExt, AsyncSeekExt, AsyncWriteExt, BufReader, SeekFrom};
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct FileStorage<SE> {
    file_path: String,
    serializer: SE,
    db_file: Mutex<File>,
}

#[async_trait]
impl<SE> Storage for FileStorage<SE>
where
    for<'de> SE: Serializer<'de> + Debug + Sync + Send,
{
    async fn new(db_name: &str) -> Result<Self> {
        let serializer = SE::default();
        let extension = match serializer.format() {
            Serializers::Bin(st) => st.clone(),
            Serializers::Json(st) => st.clone(),
            Serializers::Yaml(st) => st.clone(),
            Serializers::Ron(st) => st.clone(),
        };

        let db_path = format!("{}{}", db_name, extension);

        let file = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(&db_path)
            .await?;

        Ok(Self {
            serializer: SE::default(),
            file_path: db_path,
            db_file: Mutex::new(file),
        })
    }

    async fn load<T>(&self) -> Result<RedDbHM>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq + Send + Sync,
    {
        let mut map: RedDbHM = HashMap::new();
        let mut file = self.db_file.lock().await;
        let reader = BufReader::new(&mut *file);
        let mut lines = reader.lines();

        while let Some(line) = lines.next_line().await? {
            let byte_str = line.into_bytes();
            let document: Document<T> = self
                .serializer
                .deserialize(&byte_str)
                .map_err(|e| RedDbError::Deserialize(e.to_string()))?;

            match document._st {
                Status::De => {
                    map.remove(&document._id);
                }
                _ => {
                    let serialized = self
                        .serializer
                        .serialize(&document.data)
                        .map_err(|e| RedDbError::Serialize(e.to_string()))?;
                    map.entry(document._id).or_insert(serialized);
                }
            }
        }

        self.compact_data::<T>(&map).await?;

        Ok(map)
    }

    async fn persist<T>(&self, data: &[Document<T>]) -> Result<()>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + Sync,
    {
        let mut serialized = Vec::new();
        for doc in data {
            let bytes = self
                .serializer
                .serialize::<Document<T>>(doc)
                .map_err(|e| RedDbError::Serialize(e.to_string()))?;
            serialized.extend(bytes);
        }
        self.append(&serialized).await
    }
}

impl<SE> FileStorage<SE>
where
    for<'de> SE: Serializer<'de> + Debug,
{
    pub async fn compact_data<T>(&self, data: &RedDbHM) -> Result<()>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
    {
        let mut bytes = Vec::new();
        for (id, raw) in data {
            let value: T = self
                .serializer
                .deserialize(raw)
                .map_err(|e| RedDbError::Deserialize(e.to_string()))?;
            let doc = Document::new(*id, value, Status::In);
            let serialized = self
                .serializer
                .serialize(&doc)
                .map_err(|e| RedDbError::Serialize(e.to_string()))?;
            bytes.extend(serialized);
        }
        self.flush_data(&self.file_path, &bytes).await
    }

    async fn flush_data<P: AsRef<Path>>(&self, path: P, data: &[u8]) -> Result<()> {
        let mut file = File::create(path).await?;
        file.set_len(0).await?;
        file.seek(SeekFrom::Start(0)).await?;
        file.write_all(data).await?;
        file.sync_all().await?;
        Ok(())
    }

    async fn append(&self, data: &[u8]) -> Result<()> {
        let mut file = self.db_file.lock().await;
        file.seek(SeekFrom::End(0)).await?;
        file.write_all(data).await?;
        file.sync_all().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, Serialize, PartialEq, Deserialize)]
    struct TestStruct {
        foo: String,
    }
}
