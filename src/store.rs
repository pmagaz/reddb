use super::record::{Document, Record};
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
use uuid::Uuid;

pub type RDHM<T> = HashMap<Uuid, Mutex<Record<T>>>;
pub type Read<'a, T> = RwLockReadGuard<'a, RDHM<T>>;
pub type Write<'a, T> = RwLockWriteGuard<'a, RDHM<T>>;

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

  // pub fn insert(&self, value: T) -> Uuid {
  //   let mut store = self.to_write();
  //   let id = Uuid::new_v4();
  //   let doc = Mutex::new(Record {
  //     _id: id,
  //     data: value,
  //   });
  //   let _result = store.insert(id, doc);
  //   id
  // }

  // pub fn find_by_id<'a>(
  //   &'a self,
  //   // data: &'a RwLockReadGuard<RDHM<Record<T>>>,
  //   id: &'a Uuid,
  // ) -> MutexGuard<Record<T>> {
  //   let mut store = self.to_read();

  //   let value = store.get(&id).unwrap();
  //   let guard = value.lock().unwrap();
  //   guard
  // }
}
