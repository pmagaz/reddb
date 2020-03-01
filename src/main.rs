use serde_json::{json, Value};

extern crate reddb;
use reddb::RedDb;

fn main() {
  let json_db = RedDb::<serde_json::Value>::new();
  let id = json_db.insert(json!({ "leches": true}));
  let result = json_db.find_one(&id);
  println!("{:?}", result);

  // let json_db = RedDb::<ron::Value>::new();
  // let ron_doc = ron::de::from_str("Game: true").unwrap();
  // let id = json_db.insert(ron_doc);
  // let result = json_db.find_one(&id);
  // println!("{:?}", result);
}
