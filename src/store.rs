use std::collections::HashMap;
use std::sync::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};
use uuid::Uuid;

pub type RDHM = HashMap<Uuid, Mutex<Vec<u8>>>;
pub type Read<'a> = RwLockReadGuard<'a, RDHM>;
pub type Write<'a> = RwLockWriteGuard<'a, RDHM>;

#[derive(Debug)]
pub struct Store {
  pub data: RwLock<RDHM>,
}

impl Store {
  pub fn new() -> Self {
    let hm = HashMap::new();
    Self {
      data: RwLock::new(hm),
    }
  }

  pub fn to_read(&self) -> RwLockReadGuard<RDHM> {
    let read = self.data.read().unwrap();
    read
  }

  pub fn to_write(&self) -> RwLockWriteGuard<RDHM> {
    let write = self.data.write().unwrap();
    write
  }
}
