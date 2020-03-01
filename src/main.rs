use serde_json::{json, Value};

extern crate reddb;
use reddb::RedDb;

fn main() {
  let json_db = RedDb::<serde_json::Value>::new();
  let id = json_db.insert(json!({ "leches": true}));
  let result = json_db.find_one(&id);
  println!("{:?}", result);

  let ron_db = RedDb::<ron::Value>::new();
  let ron_doc = ron::de::from_str("Game").unwrap();
  let id = ron_db.insert(ron_doc);
  let result = ron_db.find_one(&id);
  println!("{:?}", result);
}
