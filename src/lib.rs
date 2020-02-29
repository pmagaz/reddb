use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
use uuid::Uuid;
mod record;
mod store;
mod store_handler;
use record::{Document, Record};
use store::Store;
mod storage;
use std::io::{BufRead, Seek, SeekFrom, Write};
use storage::Storage;
use store_handler::{find_by_id, insert};
pub struct RedDb<T> {
  pub store: Store<T>,
  pub storage: Storage,
}

impl<T> RedDb<T>
where
  T: Clone + Serialize,
{
  pub fn new() -> Self {
    Self {
      store: Store::<T>::new(),
      storage: Storage::new(".db2").unwrap(),
    }
  }

  pub fn insert(&self, value: T) -> Uuid {
    let mut store = self.store.to_write();
    insert::<T>(&mut store, value)
  }

  // pub fn log<'a>(&'a self, doc: &'a MutexGuard<Record<T>>) -> &'a MutexGuard<Record<T>>
  // where
  //   T: Serialize,
  // {
  //   let mut storage = self.storage.file.lock().unwrap();
  //   storage.seek(SeekFrom::End(0)).unwrap();
  //   storage.write_all(&operation_log).unwrap();
  //   storage.sync_all().unwrap();
  // }

  pub fn find_one(&self, id: &Uuid) -> T {
    let store = self.store.to_read();
    let result = find_by_id::<T>(&store, &id);
    //self.leches(&result);
    let data = result.get_data();
    let serialized = serde_json::to_vec(data).unwrap();
    // let docs: Vec<&T> = Vec::new();
    // docs.push(serialized);
    self.storage.log(&serialized);
    data.clone()
  }
}
