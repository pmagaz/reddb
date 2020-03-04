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
use store_handler::Handler;

pub type ReadJson<T> = RedDb<Document<T>>;

#[derive(Debug)]
pub struct RedDb<T> {
  pub store: Store<Document<T>>,
  pub handler: Handler,
  pub storage: Storage,
}

impl<'a, T> RedDb<T>
where
  T: Clone + Serialize + Deserialize<'a> + Debug,
{
  pub fn new() -> Self {
    Self {
      handler: Handler {},
      store: Store::<Document<T>>::new(),
      storage: Storage::new(".db").unwrap(),
    }
  }

  pub fn insert(&self, value: T) -> Uuid {
    let mut store = self.store.to_write();
    let id = Uuid::new_v4();
    let doc = Document::new(id, value);
    let result = self.handler.insert::<T, Document<T>>(&mut store, doc);
    result
  }

  pub fn find_one(&self, id: &Uuid) -> Document<T> {
    let store = self.store.to_read();
    let doc = self.handler.find_key::<Document<T>>(&store, &id);
    doc.to_owned()
    //doc.get_data().clone()
  }

  pub fn find_all(&self, value: T) -> Vec<Document<T>> {
    let store = self.store.to_read();
    let docs = self
      .handler
      .find_from_value::<T, Document<T>>(&store, value);
    docs
  }

  pub fn delete_one(&self, id: &Uuid) -> Document<T> {
    let mut store = self.store.to_write();
    let doc = self.handler.delete_key::<T, Document<T>>(&mut store, &id);
    doc
  }

  pub fn delete_all(&self, value: T) -> Vec<Document<T>> {
    let store = self.store.to_read();

    let docs = self
      .handler
      .find_from_value::<T, Document<T>>(&store, value);

    let deleted = docs.iter().map(|doc| self.delete_one(doc.get_id()));

    docs
  }
}
