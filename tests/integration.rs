use rdstore::RonStore;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::io::Error;
use std::path::Path;

type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Clone, Debug, Serialize, PartialEq, Deserialize)]
struct MyStruct {
  foo: String,
}

impl fmt::Display for MyStruct {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "({})", self)
  }
}

fn setup() -> Result<()> {
  if Path::new(".ron.db").exists() {
    fs::remove_file(".ron.db").unwrap();
  }
  Ok(())
}

#[test]
fn find() -> Result<()> {
  setup().unwrap();
  let store = RonStore::new::<MyStruct>().unwrap();
  let search = MyStruct {
    foo: String::from("hi"),
  };

  let id = store.insert(search.clone()).unwrap();
  let result: MyStruct = store.find(&id).unwrap();
  assert_eq!(result, search);
  Ok(())
}

#[test]
fn find_many() -> Result<()> {
  setup().unwrap();
  let store = RonStore::new::<MyStruct>().unwrap();

  let one = MyStruct {
    foo: String::from("one"),
  };

  let two = MyStruct {
    foo: String::from("two"),
  };

  let many = vec![one.clone(), one.clone(), two.clone()];
  let id = store.insert_many(many).unwrap();
  let result = store.find_many(&one).unwrap();
  assert_eq!(result.len(), 2);
  Ok(())
}

#[test]
fn delete() -> Result<()> {
  setup().unwrap();
  let store = RonStore::new::<MyStruct>().unwrap();
  let search = MyStruct {
    foo: String::from("hi"),
  };

  let id = store.insert(search.clone()).unwrap();
  let deleted = store.delete(&id).unwrap();
  assert_eq!(deleted, true);

  let not_deleted = store.delete(&id).unwrap();
  assert_eq!(not_deleted, false);
  Ok(())
}

#[test]
fn delete_many() -> Result<()> {
  setup().unwrap();
  let store = RonStore::new::<MyStruct>().unwrap();

  let one = MyStruct {
    foo: String::from("one"),
  };

  let two = MyStruct {
    foo: String::from("two"),
  };

  let many = vec![one.clone(), one.clone(), two.clone()];
  let id = store.insert_many(many).unwrap();
  let deleted = store.delete_many(&one).unwrap();
  assert_eq!(deleted, 2);

  let not_deleted = store.delete_many(&one).unwrap();
  assert_eq!(not_deleted, 0);
  Ok(())
}

#[test]
fn update() -> Result<()> {
  setup().unwrap();
  let store = RonStore::new::<MyStruct>().unwrap();
  let original = MyStruct {
    foo: String::from("hi"),
  };

  let updated = MyStruct {
    foo: String::from("bye"),
  };

  let id = store.insert(original.clone()).unwrap();
  store.update(&id, updated.clone()).unwrap();
  let result: MyStruct = store.find(&id).unwrap();
  assert_eq!(result, updated);
  Ok(())
}

#[test]
fn update_many() -> Result<()> {
  setup().unwrap();
  let store = RonStore::new::<MyStruct>().unwrap();

  let one = MyStruct {
    foo: String::from("one"),
  };

  let two = MyStruct {
    foo: String::from("two"),
  };

  let many = vec![one.clone(), one.clone(), two.clone()];
  let id = store.insert_many(many).unwrap();
  let updated = store.update_many(&one, &two).unwrap();
  assert_eq!(updated, 2);
  let result = store.find_many(&two).unwrap();
  assert_eq!(result.len(), 3);
  Ok(())
}
