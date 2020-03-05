use serde::{Deserialize, Serialize};
use uuid::Uuid;
mod document;
mod store;
mod store_handler;
use document::{Doc, Document};
use store::Store;
mod json_ser;
mod status;
mod storage;
pub use json_ser::{DeSerializer, DeserializeOwned, Json};
//use serde_json::Serializer;
use std::fmt::Debug;
use storage::Storage;
use store_handler::Handler;

//pub type ReadJson<T> = RedDb<Document<T>>;
pub type JsonSerializer = Json;

#[derive(Debug)]
pub struct RedDb<T, DS> {
  pub store: Store<Document<T>>,
  pub handler: Handler<DS>,
  //pub serializer: S,
  pub storage: Storage,
}

impl<'a, T, DS> RedDb<T, DS>
where
  T: Clone + Serialize + Deserialize<'a> + Debug,
  DS: DeSerializer<'a, T> + Debug + Clone,
{
  pub fn new() -> Self {
    Self {
      //serializer: Json,
      handler: Handler::<DS> {
        serializer: DS::default(),
      },
      store: Store::<Document<T>>::new(),
      storage: Storage::new(".db").unwrap(),
    }
  }

  pub fn insert(&self, value: T) -> Uuid {
    let mut store = self.store.to_write();
    let doc = Document::new(value);
    let result = self.handler.insert::<T, Document<T>>(&mut store, doc);
    result
  }

  pub fn find_one(&self, id: &Uuid) -> Document<T> {
    let store = self.store.to_read();
    let doc = self.handler.find_key::<Document<T>>(&store, &id);
    doc.to_owned()
  }

  pub fn update_one(&self, id: &Uuid, new_value: T) -> Document<T> {
    let mut store = self.store.to_write();
    let doc = self
      .handler
      .update_key::<T, Document<T>>(&mut store, &id, new_value);
    doc.to_owned()
  }

  pub fn delete_one(&self, id: &Uuid) -> Document<T> {
    let mut store = self.store.to_write();
    let doc = self.handler.delete_key::<T, Document<T>>(&mut store, &id);
    doc
  }

  pub fn find_all(&self, query: T) -> Vec<Document<T>> {
    let store = self.store.to_read();
    let docs = self
      .handler
      .find_from_value::<T, Document<T>>(&store, query);
    docs
  }

  pub fn update_all(&self, query: T, new_value: T) -> Vec<Document<T>> {
    let mut store = self.store.to_write();
    let docs = self
      .handler
      .update_from_value::<T, Document<T>>(&mut store, query, new_value);
    docs
  }

  pub fn delete_all(&self, query: T) -> Vec<Document<T>> {
    let store = self.store.to_read();

    let docs = self
      .handler
      .find_from_value::<T, Document<T>>(&store, query);

    let deleted = docs.iter().map(|doc| self.delete_one(doc.get_id()));
    docs
  }
}
