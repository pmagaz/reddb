use std::collections::HashMap;
use std::sync::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};
use uuid::Uuid;

pub type RDHM<T> = HashMap<Uuid, Mutex<T>>;
pub type Read<'a, T> = RwLockReadGuard<'a, RDHM<T>>;
pub type Write<'a, T> = RwLockWriteGuard<'a, RDHM<T>>;

#[derive(Debug)]
pub struct Store<T> {
  pub data: RwLock<RDHM<T>>,
}

impl<T> Store<T> {
  pub fn new() -> Self {
    let hm = HashMap::new();
    Self {
      data: RwLock::new(hm),
    }
  }

  pub fn to_read(&self) -> RwLockReadGuard<RDHM<T>> {
    let read = self.data.read().unwrap();
    read
  }

  pub fn to_write(&self) -> RwLockWriteGuard<RDHM<T>> {
    let write = self.data.write().unwrap();
    write
  }
}
