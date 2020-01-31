use owning_ref::MutexGuardRef;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use std::sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard};

use uuid::Uuid;
type StoreHashMap<T> = HashMap<Uuid, T>;

#[derive(Debug)]
pub struct Document {
  pub data: Value,
}

pub struct Store<T> {
  pub store: RwLock<Arc<Mutex<StoreHashMap<T>>>>,
}

impl<T> Store<T> {
  pub fn new() -> Self {
    let mut hm = HashMap::new();
    let map: Arc<Mutex<StoreHashMap<T>>> = Arc::new(Mutex::new(hm));
    Self {
      //store: map,
      store: RwLock::new(map),
    }
  }

  pub fn find_by_id<'a, 'b, 'c>(
    &'b self,
    map: &'a RwLockReadGuard<Arc<Mutex<StoreHashMap<T>>>>,
    id: &'c Uuid,
  ) -> MutexGuardRef<'a, StoreHashMap<T>, T> {
    let guard = map.lock().unwrap();
    MutexGuardRef::new(guard).map(|mg| {
      mg.get(id)
        .ok_or_else(|| Error::new(ErrorKind::NotFound, "Not found"))
        .unwrap()
    })
  }

  pub fn find_one<'a, 'b, 'c>(&'b self, id: &'c Uuid) -> MutexGuardRef<'a, StoreHashMap<T>, T> {
    let read = self.store.read().unwrap();
    let document = self.find_by_id(&read, &id);
    document
  }

  // pub fn insert<'a>(&self, key: Uuid, val: T) -> Option<Arc<Mutex<T>>> {
  //   let mut write = self.store.write().unwrap();
  //   write.insert(key, val)
  // }
}

fn main() {
  let store = Store::<Document>::new();
  let id = Uuid::new_v4();
  let doc = Document {
    data: json!({"name":"Peter"}),
  };
  //store.insert(id, doc);
  let document = store.find_one(&id);
  println!("{:?}", document);
}
