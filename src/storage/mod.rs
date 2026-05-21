use crate::document::Document;
use crate::error::Result;
use crate::wal::WalOp;
use crate::RedDbHM;
use core::fmt::Debug;
use serde::{Deserialize, Serialize};

mod file;

pub use file::FileStorage;

#[async_trait::async_trait]
pub trait Storage {
    async fn new(db_name: &str) -> Result<Self>
    where
        Self: Sized;

    async fn load<T>(&self) -> Result<RedDbHM>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq + Send + Sync;

    async fn persist<T>(&self, records: &[Document<T>], op: WalOp) -> Result<()>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + Send + Sync + Clone;
}
