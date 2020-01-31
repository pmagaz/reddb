use super::document::Document;
use super::document::Leches;
use super::store::{ReadGuard, RedDbHashMap, Result, WriteGuard};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::Lines;
use std::marker::Sized;
use std::sync::{Mutex, MutexGuard, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};
use uuid::Uuid;
pub(crate) trait JsonStore: Send + Sync {
  fn new(a: Lines<&[u8]>) -> Result<Self>
  where
    Self: Sized;

  fn find_id<T>(&self, store: RwLockReadGuard<HashMap<Uuid, Mutex<T>>>, id: &Value) -> T;
  fn to_read(&self) -> Result<ReadGuard<RedDbHashMap>>;
  fn to_write(&self) -> Result<WriteGuard<RedDbHashMap>>;
  fn get_id<'a>(&self, query: &'a Value) -> Result<&'a str>;
  fn get_uuid(&self, query: &Value) -> Result<Uuid>;
}
