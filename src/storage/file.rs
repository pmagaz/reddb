use async_trait::async_trait;
use core::fmt::Debug;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use super::Storage;
use crate::document::Document;
use crate::error::{RedDbError, Result};
use crate::serializer::Serializer;
use crate::wal::{WalEntry, WalOp};
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
    SE: Serializer + Debug + Sync + Send,
{
    async fn new(db_name: &str) -> Result<Self> {
        let serializer = SE::default();
        let db_path = format!("{}{}", db_name, serializer.format_id().extension());

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
            let bytes = line.into_bytes();
            let entry: WalEntry = self
                .serializer
                .deserialize(&bytes)
                .map_err(|e| RedDbError::Deserialize(e.to_string()))?;

            if entry.is_delete() {
                map.remove(&entry.id);
            } else {
                // Always overwrite: Update entries must replace a prior Insert
                map.insert(entry.id, entry.payload);
            }
        }

        self.compact_data::<T>(&map).await?;

        Ok(map)
    }

    async fn persist<T>(&self, data: &[Document<T>], op: WalOp) -> Result<()>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + Sync + Clone,
    {
        let mut bytes = Vec::new();
        for doc in data {
            let payload = if op == WalOp::Delete {
                Vec::new()
            } else {
                self.serializer
                    .serialize(&doc.data)
                    .map_err(|e| RedDbError::Serialize(e.to_string()))?
            };
            let entry = match op {
                WalOp::Insert => WalEntry::insert(doc.id, payload),
                WalOp::Update => WalEntry::update(doc.id, payload),
                WalOp::Delete => WalEntry::delete(doc.id),
            };
            let serialized = self
                .serializer
                .serialize(&entry)
                .map_err(|e| RedDbError::Serialize(e.to_string()))?;
            bytes.extend(serialized);
        }
        self.append(&bytes).await
    }
}

impl<SE> FileStorage<SE>
where
    SE: Serializer + Debug,
{
    pub async fn compact_data<T>(&self, data: &RedDbHM) -> Result<()>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
    {
        let mut bytes = Vec::new();
        for (id, payload) in data {
            let entry = WalEntry::insert(*id, payload.clone());
            let serialized = self
                .serializer
                .serialize(&entry)
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
