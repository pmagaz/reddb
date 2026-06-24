use std::collections::HashMap;
use std::fmt::Debug;
use std::io::BufRead;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{RedDbError, Result};
use crate::serializer::Serializer;
use crate::storage::{FileStorage, Storage};
use crate::wal::WalOp;
use crate::{DbConfig, RedDb};

// Internal v1 document shape for deserialization.
// Fields and struct name match what v1 serde produced on disk.
#[derive(Serialize, Deserialize)]
#[serde(rename = "Document")]
struct V1Document<T> {
    _id: Uuid,
    data: T,
    _st: V1Status,
}

#[derive(Serialize, Deserialize, PartialEq)]
enum V1Status {
    In,
    Up,
    De,
}

/// Migrate a v1 database file to v2 format, preserving original document UUIDs.
///
/// Reads `v1_path` line by line, replays all WAL operations in order (In/Up = upsert,
/// De = delete), then writes every surviving document into a new v2 database named
/// `v2_name` using serializer `SE`.
///
/// Returns the number of documents written to the v2 database.
///
/// # Notes
/// - Binary (`.bin`) v1 files cannot be migrated: v1's line-delimited format
///   corrupted binary records. Use a text format (JSON, RON, YAML) instead.
/// - Call this function exactly once per database. Running it a second time
///   will append duplicate records to an already-populated v2 file.
///
/// # Example
/// ```ignore
/// reddb::migrate::from_v1::<MyType, reddb::serializer::Ron>("users.ron", "users_v2").await?;
/// ```
#[allow(private_bounds)]
pub async fn from_v1<T, SE>(v1_path: &str, v2_name: &str) -> Result<usize>
where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Clone + PartialEq + Send + Sync + 'static,
    SE: Serializer + Debug + 'static,
    FileStorage<SE>: Storage + Debug + Send + Sync + 'static,
{
    let serializer = SE::default();
    let mut live: HashMap<Uuid, Vec<u8>> = HashMap::new();

    let file = std::fs::File::open(v1_path)?;
    let reader = std::io::BufReader::new(file);

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let v1doc: V1Document<T> = serializer
            .deserialize(trimmed.as_bytes())
            .map_err(|e| RedDbError::Deserialize(e.to_string()))?;

        if v1doc._st == V1Status::De {
            live.remove(&v1doc._id);
        } else {
            let raw = serializer
                .serialize(&v1doc.data)
                .map_err(|e| RedDbError::Serialize(e.to_string()))?;
            live.insert(v1doc._id, raw);
        }
    }

    if live.is_empty() {
        return Ok(0);
    }

    let db: RedDb<SE, FileStorage<SE>> = RedDb::open::<T>(DbConfig::new(v2_name)).await?;

    let ops: Vec<(WalOp, Uuid, Vec<u8>)> = live
        .iter()
        .map(|(id, raw)| (WalOp::Insert, *id, raw.clone()))
        .collect();

    let count = ops.len();

    {
        let mut data = db.write_lock().await?;
        for (_, id, raw) in &ops {
            data.insert(*id, raw.clone());
        }
    }

    db.storage_persist_raw(&ops).await?;

    Ok(count)
}
