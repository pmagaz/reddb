use serde::{Deserialize, Serialize};
use uuid::Uuid;
mod document;
mod store;
mod store_handler;
use document::{Doc, Document};
use store::Store;
mod status;
mod storage;
use status::Status;
use std::fmt::Debug;
use storage::Storage;
use store_handler::{find_key, find_value, insert};

pub type ReadJson<T> = RedDb<Document<T>>;

#[derive(Debug)]
pub struct RedDb<T> {
  pub store: Store<Document<T>>,
  pub storage: Storage,
}

impl<'a, T> RedDb<T>
where
  T: Clone + Serialize + Deserialize<'a> + Debug,
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
    let doc = Document::new(id, value, Status::NotSaved);
    let result = insert::<T, Document<T>>(&mut store, doc);
    result
  }

  pub fn find_one(&self, id: &Uuid) -> T {
    let store = self.store.to_read();
    let doc = find_key::<Document<T>>(&store, &id);
    self.storage.write(&doc.as_u8());
    doc.get_data().clone()
  }

  pub fn find(&self, value: T) -> Vec<Document<T>> {
    let store = self.store.to_read();
    let docs = find_value::<T, Document<T>>(&store, value);
    //self.storage.write(&doc.as_u8());
    docs
    //11
  }
}
