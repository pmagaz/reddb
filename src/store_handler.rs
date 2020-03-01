use super::record::{Record, RedDbRecord};
use super::status::Status;
use super::store::{Read, Write};

use std::sync::{Mutex, MutexGuard};
use uuid::Uuid;

pub fn insert<T>(store: &mut Write<T>, value: T) -> Uuid {
  let id = Uuid::new_v4();
  let doc = Mutex::new(RedDbRecord {
    _id: id,
    data: value,
    status: Status::NotSaved,
  });
  let _result = store.insert(id, doc);
  id
}

pub fn find_by_id<'a, T>(store: &'a Read<T>, id: &'a Uuid) -> MutexGuard<'a, RedDbRecord<T>> {
  let value = store.get(&id).unwrap();
  let guard = value.lock().unwrap();
  guard
}
