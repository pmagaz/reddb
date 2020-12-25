use crate::error::Result;
use crate::RedDbHM;
use async_trait::async_trait;
use core::fmt::Debug;
use serde::{Deserialize, Serialize};
use std::marker::Sized;

mod file;
use crate::document::Document;

pub use file::FileStorage;

#[async_trait]
pub trait Storage {
  fn new(db_name: &str) -> Result<Self>
  where
    Self: Sized;
  fn load<T>(&self) -> Result<RedDbHM>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq;
  async fn persist<T>(&self, records: &[Document<T>]) -> Result<()>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Sync;
}
