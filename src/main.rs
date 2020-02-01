use owning_ref::MutexGuardRef;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use std::sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard};

use uuid::Uuid;
type StoreHashMap<T> = HashMap<Uuid, T>;

#[derive(Debug, Clone)]
pub struct Document {
  pub data: Value,
  //    data: RwLock<Hashmap>,
}

pub struct Store<T> {
  pub store: Arc<RwLock<Mutex<StoreHashMap<T>>>>,
}

impl<T> Store<T> {
  pub fn new() -> Self {
    let hm = HashMap::new();
    let map: RwLock<Mutex<StoreHashMap<T>>> = RwLock::new(Mutex::new(hm));
    Self {
      store: Arc::new(map),
    }
  }

  pub fn find_by_id<'a, 'b: 'a, 'c>(
    &'b self,
    store: &'a RwLockReadGuard<Mutex<StoreHashMap<T>>>,
    id: &'c Uuid,
  ) -> MutexGuardRef<'a, StoreHashMap<T>, T> {
    let map = store.lock().unwrap();
    MutexGuardRef::new(map).map(|mg| {
      mg.get(id)
        .ok_or_else(|| Error::new(ErrorKind::NotFound, "Not found"))
        .unwrap()
    })
  }

  pub fn find_one<'a, 'b: 'a, 'c>(&'b self, id: &'c Uuid) -> MutexGuardRef<'a, StoreHashMap<T>, T> {
    //pub fn find_one<'a, 'b: 'a, 'c>(&'b self, id: &'c Uuid) -> Result<()> {
    let store = self.store.read().unwrap();
    let document = self.find_by_id(&store, &id);
    document.clone()
  }
}

fn main() {
  let store = Store::<Document>::new();
  let id = Uuid::new_v4();
  let doc = Document {
    data: json!({"name":"Peter"}),
  };
  let document = store.find_one(&id);
  println!("{:?}", document);
}
