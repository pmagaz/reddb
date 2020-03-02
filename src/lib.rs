use serde::{Deserialize, Serialize};
use uuid::Uuid;
mod document;
mod store;
mod store_handler;
use document::{Doc, Document};
use store::Store;
mod status;
mod storage;
use storage::Storage;
use store_handler::{find_key, insert};

pub type ReadJson<T> = RedDb<Document<T>>;

pub struct RedDb<T> {
  pub store: Store<Document<T>>,
  pub storage: Storage,
}

impl<T> RedDb<T>
where
  T: Clone + Serialize,
{
  pub fn new() -> Self {
    Self {
      store: Store::<Document<T>>::new(),
      storage: Storage::new(".db").unwrap(),
    }
  }

  pub fn insert(&self, value: T) -> Uuid {
    let mut store = self.store.to_write();
    let id = Uuid::new_v4();
    let doc = Document::new(id, value, status::Status::NotSaved);
    insert::<T, Document<T>>(&mut store, doc)
  }

  pub fn find_one(&self, id: &Uuid) -> T {
    let store = self.store.to_read();
    let result = find_key::<Document<T>>(&store, &id);
    self.storage.write(&result.as_u8());
    result.get_data().clone()
  }
}
