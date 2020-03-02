use serde_json::{json, Value};

extern crate reddb;
use reddb::RedDb;

fn main() {
  let db = RedDb::<Value>::new();
  let id = db.insert(json!({ "leches": true}));
  let result = db.find_one(&id);
  println!("FIND_ONE {:?}", result);
  db.insert(json!({"name":"record1", "leches": 11}));
  let result = db.find(json!({"name":"record1", "leches": 11}));

  // let ron_db = RedDb::<ron::Value>::new();
  // let ron_doc = ron::de::from_str("Game").unwrap();
  // let id = ron_db.insert(ron_doc);
  // let result = ron_db.find_one(&id);
  // println!("{:?}", result);
}
