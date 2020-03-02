use super::document::{Doc, Document};
use super::status::Status;
use super::store::{Read, Write};

use std::sync::{Mutex, MutexGuard};
use uuid::Uuid;

pub fn insert<L, T>(store: &mut Write<T>, doc: T) -> Uuid
where
  T: Doc<L>,
{
  let _id = **&doc.get_id();
  let _result = store.insert(_id, Mutex::new(doc));
  _id
}

pub fn find_key<'a, T>(store: &'a Read<T>, id: &'a Uuid) -> MutexGuard<'a, T> {
  let value = store.get(&id).unwrap();
  let guard = value.lock().unwrap();
  guard
}
