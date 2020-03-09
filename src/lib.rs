use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use uuid::Uuid;

mod document;
mod store;
mod store_handler;
use document::{Doc, Document};
use store::Store;
mod deserializer;
mod status;
mod storage;
use storage::Storage;

pub use deserializer::{DeSerializer, JsonSerializer};
use store_handler::Handler;

#[derive(Debug)]
pub struct RedDb<T, DS> {
  pub store: Store<Doc<T>>,
  pub handler: Handler,
  pub serializer: DS,
  pub storage: Storage,
}

impl<'a, T, DS> RedDb<T, DS>
where
  for<'de> T: Serialize + Deserialize<'de> + Debug + Clone,
  for<'de> DS: DeSerializer<'de, Doc<T>> + Debug + Clone,
{
  pub fn new() -> Self {
    Self {
      serializer: DS::default(),
      handler: Handler::default(),
      store: Store::<Doc<T>>::new(),
      storage: Storage::new(".db").unwrap(),
    }
  }

  pub fn insert(&self, value: T) -> Uuid {
    let mut store = self.store.to_write();
    let doc = Doc::new(value);
    let result = self.handler.insert(&mut store, doc);
    result
  }

  pub fn find_one(&self, id: &Uuid) -> Doc<T> {
    let store = self.store.to_read();
    let doc = self.handler.find_key(&store, &id);
    doc.to_owned()
  }

  pub fn update_one(&self, id: &Uuid, new_value: T) -> Doc<T> {
    let mut store = self.store.to_write();
    let doc = self.handler.update_key(&mut store, &id, new_value);
    doc.to_owned()
  }

  pub fn delete_one(&self, id: &Uuid) -> Doc<T> {
    let mut store = self.store.to_write();
    let doc = self.handler.delete_key(&mut store, &id);
    //let leches = self.serializer.serializer(&doc);
    doc
  }

  pub fn find_all(&self, query: T) -> Vec<Doc<T>> {
    let store = self.store.to_read();
    let docs = self
      .handler
      .find_from_value(&store, &self.serializer, query);
    docs
  }

  pub fn update_all(&self, query: T, new_value: T) -> Vec<Doc<T>> {
    let mut store = self.store.to_write();
    let serializer = &self.serializer;
    let docs = self
      .handler
      .update_from_value(&mut store, serializer, query, new_value);
    docs
  }

  pub fn delete_all(&self, query: T) -> Vec<Doc<T>> {
    let store = self.store.to_read();

    let docs = self
      .handler
      .find_from_value(&store, &self.serializer, query);

    docs.iter().map(|doc| self.delete_one(doc.get_id()));
    docs
  }
}
