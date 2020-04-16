use crate::error::Result;
use crate::{StoreHM, WriteOperation};
use core::fmt::Debug;
use serde::{Deserialize, Serialize};
use std::marker::Sized;

mod file;
pub use file::FileStorage;

pub trait Storage {
  fn new<T>() -> Result<Self>
  where
    Self: Sized;
  fn load_content<T>(&self) -> Result<StoreHM>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq;
  fn save<T>(&self, docs: Vec<WriteOperation<T>>) -> Result<()>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug;
  fn save_one<T>(&self, doc: WriteOperation<T>) -> Result<()>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug;
}
