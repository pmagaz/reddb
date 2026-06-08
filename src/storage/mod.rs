use crate::document::Document;
use crate::error::Result;
use crate::wal::WalOp;
use crate::RedDbHM;
use core::fmt::Debug;
use serde::{Deserialize, Serialize};

mod file;
mod mem;

pub use file::FileStorage;
pub use mem::MemStorage;

#[async_trait::async_trait]
pub trait Storage {
    async fn new(db_name: &str, compaction_ratio: f64) -> Result<Self>
    where
        Self: Sized;

    async fn load<T>(&self) -> Result<RedDbHM>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq + Send + Sync;

    async fn persist<T>(&self, records: &[Document<T>], op: WalOp) -> Result<()>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + Send + Sync + Clone;

    /// Rewrite the storage with exactly one Insert record per live document.
    async fn compact(&self, data: &RedDbHM) -> Result<()>;

    /// Size of the backing store in bytes (0 for in-memory backends).
    async fn file_size(&self) -> Result<u64>;
}
