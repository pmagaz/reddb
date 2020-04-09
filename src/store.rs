use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use std::result;
use std::sync::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
use uuid::Uuid;

type ByteString = Vec<u8>;
pub type Result<T> = result::Result<T, std::io::Error>;

pub type RDHM = HashMap<Uuid, Mutex<ByteString>>;
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

  pub fn to_read(&self) -> Read {
    let read = self.data.read().unwrap();
    read
  }

  pub fn to_write(&self) -> Write {
    let write = self.data.write().unwrap();
    write
  }
}
