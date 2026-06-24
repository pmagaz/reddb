use async_trait::async_trait;
use core::fmt::Debug;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io;

use super::Storage;
use crate::document::Document;
use crate::error::{RedDbError, Result};
use crate::serializer::{FormatId, Serializer};
use crate::wal::WalOp;
use crate::RedDbHM;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, SeekFrom};
use tokio::sync::Mutex;
use uuid::Uuid;

/// 32-byte file header layout:
/// [0..8]   magic   b"REDDB\x00\x02\x00"
/// [8..10]  version u16 LE (2)
/// [10]     format  u8  (FormatId discriminant)
/// [11..32] reserved (zeroed)
const HEADER_LEN: u64 = 32;
const MAGIC: &[u8; 8] = b"REDDB\x00\x02\x00";
const VERSION: u16 = 2;

/// Per-record layout: [u32 LE payload_len][u8 op][u8;16 uuid][payload_len bytes]
const RECORD_OVERHEAD: usize = 21; // 4 + 1 + 16

fn build_header(format: FormatId) -> [u8; 32] {
    let mut h = [0u8; 32];
    h[0..8].copy_from_slice(MAGIC);
    h[8..10].copy_from_slice(&VERSION.to_le_bytes());
    h[10] = format as u8;
    h
}

fn verify_header(header: &[u8; 32], expected: FormatId) -> Result<()> {
    if &header[0..8] != MAGIC {
        return Err(RedDbError::DataCorrupted);
    }
    let version = u16::from_le_bytes(header[8..10].try_into().unwrap());
    if version != VERSION {
        return Err(RedDbError::DataCorrupted);
    }
    if header[10] != expected as u8 {
        return Err(RedDbError::DataCorrupted);
    }
    Ok(())
}

/// Byte size the file would have after compacting `data`.
fn compacted_size(data: &RedDbHM) -> u64 {
    let payload_bytes: u64 = data.values().map(|v| v.len() as u64).sum();
    HEADER_LEN + data.len() as u64 * RECORD_OVERHEAD as u64 + payload_bytes
}

fn should_compact(file_size: u64, live_size: u64, ratio: f64) -> bool {
    live_size > 0 && (file_size as f64) >= (live_size as f64) * ratio
}

async fn open_append(path: &str) -> Result<File> {
    Ok(OpenOptions::new()
        .read(true)
        .append(true)
        .create(true)
        .open(path)
        .await?)
}

async fn read_records(file: &mut File) -> Result<Vec<(WalOp, Uuid, Vec<u8>)>> {
    file.seek(SeekFrom::Start(HEADER_LEN)).await?;
    let mut records = Vec::new();
    let mut len_buf = [0u8; 4];

    loop {
        match file.read_exact(&mut len_buf).await {
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e.into()),
        }

        let payload_len = u32::from_le_bytes(len_buf) as usize;

        let mut meta = [0u8; 17]; // 1 op + 16 uuid
        file.read_exact(&mut meta).await?;

        let op = match meta[0] {
            0x01 => WalOp::Insert,
            0x02 => WalOp::Update,
            0x03 => WalOp::Delete,
            _ => return Err(RedDbError::DataCorrupted),
        };

        let id = Uuid::from_bytes(meta[1..17].try_into().unwrap());

        let mut payload = vec![0u8; payload_len];
        file.read_exact(&mut payload).await?;

        records.push((op, id, payload));
    }

    Ok(records)
}

async fn write_record(file: &mut File, op: WalOp, id: Uuid, payload: &[u8]) -> Result<()> {
    let len = payload.len() as u32;
    let mut frame = Vec::with_capacity(RECORD_OVERHEAD + payload.len());
    frame.extend_from_slice(&len.to_le_bytes());
    frame.push(match op {
        WalOp::Insert => 0x01,
        WalOp::Update => 0x02,
        WalOp::Delete => 0x03,
    });
    frame.extend_from_slice(id.as_bytes());
    frame.extend_from_slice(payload);
    file.write_all(&frame).await?;
    Ok(())
}

#[derive(Debug)]
pub struct FileStorage<SE> {
    file_path: String,
    compaction_ratio: f64,
    serializer: SE,
    db_file: Mutex<File>,
}

#[async_trait]
impl<SE> Storage for FileStorage<SE>
where
    SE: Serializer + Debug + Sync + Send,
{
    async fn new(db_name: &str, compaction_ratio: f64) -> Result<Self> {
        let serializer = SE::default();
        let db_path = format!("{}{}", db_name, serializer.format_id().extension());
        let file = open_append(&db_path).await?;
        let storage = Self {
            serializer,
            compaction_ratio,
            file_path: db_path,
            db_file: Mutex::new(file),
        };
        storage.init_header().await?;
        Ok(storage)
    }

    #[allow(clippy::extra_unused_type_parameters)]
    async fn load<T>(&self) -> Result<RedDbHM>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq + Send + Sync,
    {
        let (map, file_size) = {
            let mut file = self.db_file.lock().await;
            let file_size = file.metadata().await?.len();
            let records = read_records(&mut file).await?;
            let mut map: RedDbHM = HashMap::new();
            for (op, id, payload) in records {
                if op == WalOp::Delete {
                    map.remove(&id);
                } else {
                    map.insert(id, payload);
                }
            }
            (map, file_size)
        };

        let live_size = compacted_size(&map);
        if should_compact(file_size, live_size, self.compaction_ratio) {
            self.compact(&map).await?;
        }

        Ok(map)
    }

    async fn persist<T>(&self, data: &[Document<T>], op: WalOp) -> Result<()>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + Sync + Clone,
    {
        let mut file = self.db_file.lock().await;
        for doc in data {
            let payload = if op == WalOp::Delete {
                Vec::new()
            } else {
                self.serializer
                    .serialize(&doc.data)
                    .map_err(|e| RedDbError::Serialize(e.to_string()))?
            };
            write_record(&mut file, op, doc.id, &payload).await?;
        }
        file.sync_data().await?;
        Ok(())
    }

    async fn compact(&self, data: &RedDbHM) -> Result<()> {
        let tmp_path = format!("{}.tmp", self.file_path);

        {
            let mut tmp = File::create(&tmp_path).await?;
            let header = build_header(self.serializer.format_id());
            tmp.write_all(&header).await?;
            for (id, payload) in data {
                write_record(&mut tmp, WalOp::Insert, *id, payload).await?;
            }
            tmp.sync_all().await?;
        }

        let mut file = self.db_file.lock().await;
        tokio::fs::rename(&tmp_path, &self.file_path).await?;
        *file = open_append(&self.file_path).await?;
        Ok(())
    }

    async fn file_size(&self) -> Result<u64> {
        let file = self.db_file.lock().await;
        Ok(file.metadata().await?.len())
    }

    async fn persist_raw(&self, records: &[(WalOp, Uuid, Vec<u8>)]) -> Result<()> {
        let mut file = self.db_file.lock().await;
        for (op, id, payload) in records {
            write_record(&mut file, *op, *id, payload).await?;
        }
        file.sync_data().await?;
        Ok(())
    }
}

impl<SE> FileStorage<SE>
where
    SE: Serializer + Debug,
{
    async fn init_header(&self) -> Result<()> {
        let mut file = self.db_file.lock().await;
        let metadata = file.metadata().await?;
        if metadata.len() == 0 {
            let header = build_header(self.serializer.format_id());
            file.write_all(&header).await?;
            file.sync_all().await?;
        } else {
            let mut header = [0u8; 32];
            file.seek(SeekFrom::Start(0)).await?;
            file.read_exact(&mut header).await?;
            verify_header(&header, self.serializer.format_id())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_header_magic_and_version() {
        let h = build_header(FormatId::Ron);
        assert_eq!(&h[0..8], MAGIC);
        assert_eq!(u16::from_le_bytes(h[8..10].try_into().unwrap()), VERSION);
        assert_eq!(h[10], FormatId::Ron as u8);
        assert!(h[11..].iter().all(|&b| b == 0));
    }

    #[test]
    fn verify_header_succeeds_for_matching_format() {
        let h = build_header(FormatId::Json);
        assert!(verify_header(&h, FormatId::Json).is_ok());
    }

    #[test]
    fn verify_header_fails_for_wrong_magic() {
        let mut h = build_header(FormatId::Bin);
        h[0] = 0xFF;
        assert!(matches!(
            verify_header(&h, FormatId::Bin),
            Err(RedDbError::DataCorrupted)
        ));
    }

    #[test]
    fn verify_header_fails_for_wrong_format() {
        let h = build_header(FormatId::Bin);
        assert!(matches!(
            verify_header(&h, FormatId::Json),
            Err(RedDbError::DataCorrupted)
        ));
    }

    #[test]
    fn verify_header_fails_for_wrong_version() {
        let mut h = build_header(FormatId::Yaml);
        h[8] = 0xFF;
        assert!(matches!(
            verify_header(&h, FormatId::Yaml),
            Err(RedDbError::DataCorrupted)
        ));
    }

    #[test]
    fn format_id_discriminants() {
        assert_eq!(FormatId::Json as u8, 0);
        assert_eq!(FormatId::Ron as u8, 1);
        assert_eq!(FormatId::Yaml as u8, 2);
        assert_eq!(FormatId::Bin as u8, 3);
    }

    #[test]
    fn compacted_size_is_header_plus_records() {
        let mut data: RedDbHM = HashMap::new();
        let id = Uuid::new_v4();
        let payload = vec![1u8; 10];
        data.insert(id, payload);
        // HEADER_LEN(32) + 1 * RECORD_OVERHEAD(21) + 10 payload bytes = 63
        assert_eq!(compacted_size(&data), 63);
    }

    #[test]
    fn should_compact_triggers_when_file_exceeds_threshold() {
        // file_size 200, live_size 50, ratio 2.0 → 200 >= 100 → compact
        assert!(should_compact(200, 50, 2.0));
        // file_size 80, live_size 50, ratio 2.0 → 80 < 100 → no compact
        assert!(!should_compact(80, 50, 2.0));
        // live_size 0 → never compact (empty db)
        assert!(!should_compact(32, 0, 2.0));
    }
}
