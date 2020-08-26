use reddb::{Document, RonDb};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::io::Error;
use std::path::Path;
use uuid::Uuid;

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
  if Path::new(".db.yaml").exists() {
    fs::remove_file(".db.yaml").unwrap();
  }
  Ok(())
}

#[test]
fn find_one() -> Result<()> {
  setup().unwrap();
  let db = RonDb::new::<MyStruct>(".db").unwrap();
  let search = MyStruct {
    foo: String::from("hi"),
  };

  let doc = db.insert_one(search.clone()).unwrap();
  let result: Document<MyStruct> = db.find_one(&doc._id).unwrap();
  println!("hola {:?}", result);
  assert_eq!(result.data, search);
  Ok(())
}

#[test]
fn find() -> Result<()> {
  setup().unwrap();
  let db = RonDb::new::<MyStruct>(".db").unwrap();

  let one = MyStruct {
    foo: String::from("one"),
  };

  let two = MyStruct {
    foo: String::from("two"),
  };

  let many = vec![one.clone(), one.clone(), two.clone()];
  let uuid = db.insert(many).unwrap();
  let result = db.find(&one).unwrap();
  assert_eq!(result.len(), 2);
  Ok(())
}

#[test]
fn delete_one() -> Result<()> {
  setup().unwrap();
  let db = RonDb::new::<MyStruct>(".db").unwrap();
  let search = MyStruct {
    foo: String::from("hi"),
  };

  let doc = db.insert_one(search.clone()).unwrap();
  let deleted = db.delete_one(&doc._id).unwrap();
  assert_eq!(deleted, true);

  let not_deleted = db.delete_one(&doc._id).unwrap();
  assert_eq!(not_deleted, false);
  Ok(())
}

#[test]
fn delete() -> Result<()> {
  setup().unwrap();
  let db = RonDb::new::<MyStruct>(".db").unwrap();

  let one = MyStruct {
    foo: String::from("one"),
  };

  let two = MyStruct {
    foo: String::from("two"),
  };

  let many = vec![one.clone(), one.clone(), two.clone()];
  let uuid = db.insert(many).unwrap();
  let deleted = db.delete(&one).unwrap();
  assert_eq!(deleted, 2);

  let not_deleted = db.delete(&one).unwrap();
  assert_eq!(not_deleted, 0);
  Ok(())
}

#[test]
fn update_one() -> Result<()> {
  setup().unwrap();
  let db = RonDb::new::<MyStruct>(".db").unwrap();
  let original = MyStruct {
    foo: String::from("hi"),
  };

  let updated = MyStruct {
    foo: String::from("bye"),
  };

  let doc = db.insert_one(original.clone()).unwrap();
  db.update_one(&doc._id, updated.clone()).unwrap();
  let result: Document<MyStruct> = db.find_one(&doc._id).unwrap();
  assert_eq!(result.data, updated);
  Ok(())
}

#[test]
fn update() -> Result<()> {
  setup().unwrap();
  let db = RonDb::new::<MyStruct>(".db").unwrap();

  let one = MyStruct {
    foo: String::from("one"),
  };

  let two = MyStruct {
    foo: String::from("two"),
  };

  let many = vec![one.clone(), one.clone(), two.clone()];
  let uuid = db.insert(many).unwrap();
  let updated = db.update(&one, &two).unwrap();
  assert_eq!(updated, 2);
  let result = db.find(&two).unwrap();
  assert_eq!(result.len(), 3);
  Ok(())
}
