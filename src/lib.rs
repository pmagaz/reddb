use serde::{Deserialize, Serialize};
use uuid::Uuid;
mod record;
mod store;
mod store_handler;
use record::{Record, RedDbRecord};
use store::Store;
mod status;
mod storage;
use serde_json::{json, Value};
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

  pub fn find_one(&self, id: &Uuid) -> T {
    let store = self.store.to_read();
    let result = find_by_id::<T>(&store, &id);
    self.storage.write(&result.as_u8());
    result.get_data().clone()
  }
}
