use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use serde_json::json;

extern crate reddb;
use reddb::RedDb;

fn main() {
    dotenv().ok();
    let mut db = RedDb::new(".db").unwrap();

    #[derive(Serialize, Deserialize)]
    struct Search {
        name: String,
    };

    let search = Search {
        name: "notsaved22".to_owned(),
    };

    // let result = db.find(&json!({"name":"notsaved11"})).unwrap();
    // println!("FIND: name {:?}", result);

    // let result = db
    //     .find(&json!({"name":"notsaved11", "leches": 45}))
    //     .unwrap();
    // println!("FIND: name & leches {:?}", result);

    // let result = db.find(&json!({"name":"notsaved11"})).unwrap();
    // println!("FIND: name {:?}", result);

    // let result = db
    //     .find_id(&json!({"_id":"e7cdef61-d09d-420a-9a3d-e485c056c6aa"}))
    //     .unwrap();
    // println!("FINDONE_ID: name {:?}", result);

    let result = db
        .update(
            json!({"name":"record2", "leches": 22}),
            json!({"name":"record22", "leches": 222222}),
        )
        .unwrap();
    println!("UPDATED: {:?}", result);

    let result = db.delete(json!({"name":"record1", "leches": 12})).unwrap();
    println!("DELETED ONE: {:?}", result);

    let result = db.delete(json!({"name":"record3", "leches": 33})).unwrap();
    println!("DELETED TWO: {:?}", result);

    // let result = db.insert(json!({"name":"record4", "leches": 44})).unwrap();
    // println!("INSERT: name & leches {:?}", result);

    db.flush_store();
}
