use serde::{Deserialize, Serialize};
use std::fmt;
extern crate rdstore;
use uuid::Uuid;

fn main() -> std::result::Result<(), failure::Error> {
  #[derive(Clone, Debug, Default, Serialize, PartialEq, Deserialize)]
  struct MyStruct {
    foo: String,
  }

  impl fmt::Display for MyStruct {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      write!(f, "({})", self)
    }
  }

  //STRUCTS
  println!("STRUCTS");
  let query = MyStruct {
    foo: String::from("hola"),
  };
  let new_value = MyStruct {
    foo: String::from("new Value"),
  };
  let new_value2 = MyStruct {
    foo: String::from("new Value"),
  };
  let new_value3 = MyStruct {
    foo: String::from("new Value3"),
  };
  let db = rdstore::RonStore::<MyStruct>::new()?;
  let _id = db.insert(query.clone()).unwrap();
  //let _id = db.insert_one(String::from("hola"));
  let id = db
    .insert(MyStruct {
      foo: String::from("holaa"),
    })
    .unwrap();
  let result = db.find(&Uuid::new_v4())?;
  println!("FIND_ONE {:?}", result);
  let result = db.find_many(&query)?;
  println!("FIND ALL {:?}", result);
  let result = db.update(&_id, new_value)?;
  println!("FIND ONE UPDATED {:?}", result);

  //let result = db.delete(&Uuid::new_v4())?;
  println!("FIND ONE DELETED {:?}", result);
  let result = db.update_many(&query, &new_value2)?;
  println!("UPDATE ALL {:?}", result);

  let result = db.update_many(&query, &new_value3)?;
  println!("UPDATE ALL {:?}", result);

  let another = MyStruct {
    foo: String::from("22"),
  };
  //let id = db.insert(another.clone());
  // let id = db.insert(another.clone());
  let result = db.delete_many(&another)?;
  println!("DELETE ALL {:?}", result);

  let arr = vec![another.clone(), another.clone()];
  let result = db.insert_many(arr)?;
  // println!("INSERT {:?}", result);
  // println!("JSON STRINGS");
  // let db = RedDb::<String, JsonSerializer>::new();
  // let query = r#"
  //       {
  //           "leches": true,
  //           "boo": 12,
  //       }"#;

  // let query2 = r#"
  //       {
  //           "leches": false,
  //           "boo": 12,
  //       }"#;

  // let id = db.insert_many(query.to_owned());
  // println!("INSERT {:?}", &id);
  // let result = db.find_one(&id);
  // println!("FIND_ONE {:?}", result);
  // let result = db.find(&query2.to_owned());
  // println!("FIND ALL {:?}", result);
  //JSON VALUES
  //println!("JSON VALUES");

  // let db2 = RedDb::<Value, JsonSerializer>::new();
  // let _id = db2.insert_many(json!({ "leches": true}));
  // let id = db2.insert_many(json!({ "leches": true, "boo": 12}));
  // let id2 = db2.insert_many(json!({ "leches": false}));
  // let result = db2.find_one(&id);
  // println!("FIND_ONE {:?}", result);
  // db2.insert_many(json!({"name":"record1", "leches": 11}));
  // let result = db2.find(json!({"name":"record1", "leches": 11}));
  // println!("FIND ALL {:?}", result);
  // db2.delete&id2);
  // let result = db2.find(json!({ "leches": false}));
  // println!("FIND DELETED ONE {:?}", result);
  // db2.delete_many(json!({ "leches": true}));
  // let result = db2.find(json!({ "leches": false}));
  // println!("FIND DELETED ALL {:?}", result);
  // let result = db2.find(json!({ "leches": true, "boo": 12}));
  // println!("FIND ALL {:?}", result);
  // let id3 = db2.insert_many(json!({ "record": true, "foo": 11}));
  // db2.update&id3, json!({ "record": "updateeeed"}));
  // let result = db2.find_one(&id3);
  // println!("UPDATED ONE {:?}", result);
  // db2.update_many(
  //   json!({ "record": "updateeeed"}),
  //   json!({ "record": "updated!"}),
  // );
  // let result = db2.find(json!({ "record": "updated!"}));
  // println!("UPDATED ALL {:?}", result);
  Ok(())
}
