use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

mod record;
mod store;
mod store_handler;
use record::Document;
use store::Store;
mod storage;
use storage::Storage;
use store_handler::{find_by_id, insert};
pub struct RedDb<T> {
  pub store: Store<T>,
  //pub storage: Storage,
}

impl<T> RedDb<T>
where
  T: Clone,
{
  pub fn new() -> Self {
    Self {
      store: Store::<T>::new(),
    }
  }

  pub fn insert(&self, value: T) -> Uuid {
    let mut store = self.store.to_write();
    insert::<T>(&mut store, value)
  }

  // pub fn leches<'a>(&'a self, doc: &'a MutexGuard<Record<T>>) -> &'a MutexGuard<Record<T>> {
  //   doc
  // }

  pub fn find_one(&self, id: &Uuid) -> T {
    let store = self.store.to_read();
    let result = find_by_id::<T>(&store, &id);
    //self.leches(&result);
    let data = result.get_data();
    data.clone()
  }
}
