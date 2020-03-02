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
use store_handler::{find_key, insert};

pub type ReadJson<T> = RedDb<RedDbRecord<T>>;

pub struct RedDb<T> {
  pub store: Store<RedDbRecord<T>>,
  pub storage: Storage,
}

impl<T> RedDb<T>
where
  T: Clone + Serialize,
{
  pub fn new() -> Self {
    Self {
      store: Store::<RedDbRecord<T>>::new(),
      storage: Storage::new(".db").unwrap(),
    }
  }

  pub fn insert(&self, value: T) -> Uuid {
    let mut store = self.store.to_write();
    let id = Uuid::new_v4();
    let record = RedDbRecord::new(id, value, status::Status::NotSaved);
    insert::<T, RedDbRecord<T>>(&mut store, record)
  }

  pub fn find_one(&self, id: &Uuid) -> T {
    let store = self.store.to_read();
    let result = find_key::<RedDbRecord<T>>(&store, &id);
    self.storage.write(&result.as_u8());
    result.get_data().clone()
  }
}
