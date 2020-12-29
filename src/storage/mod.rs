use crate::error::Result;
use crate::RedDbHM;
use core::fmt::Debug;
use serde::{Deserialize, Serialize};
use std::marker::Sized;

mod file;
use crate::document::Document;

pub use file::FileStorage;

#[async_trait::async_trait]
pub trait Storage {
    async fn new(db_name: &str) -> Result<Self>
    where
        Self: Sized;
    async fn load<T>(&self) -> Result<RedDbHM>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq + Send + Sync;
    async fn persist<T>(&self, records: &[Document<T>]) -> Result<()>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + Send + Sync;
}
