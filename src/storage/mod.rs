use crate::error::Result;
use crate::StoreHM;
use core::fmt::Debug;
use serde::{Deserialize, Serialize};
use std::marker::Sized;

mod file;
use crate::kv::KeyValue;

pub use file::FileStorage;

pub trait Storage {
  fn new<T>() -> Result<Self>
  where
    Self: Sized;
  fn load_content<T>(&self) -> Result<StoreHM>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq;
  fn save<T>(&self, docs: Vec<KeyValue<T>>) -> Result<()>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug;
  fn save_one<T>(&self, doc: KeyValue<T>) -> Result<()>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug;
}
