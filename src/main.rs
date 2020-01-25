use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use serde_json::json;

mod dstore;
mod handlers;
use dstore::DStore;
#[macro_use]
extern crate quick_error;

fn main() {
    dotenv().ok();
    let mut db = DStore::new(".db").unwrap();

    #[derive(Serialize, Deserialize)]
    struct Search {
        name: String,
    };

    let search = Search {
        name: "notsaved22".to_owned(),
    };

    let result = db.find(&json!({"name":"notsaved11"})).unwrap();
    println!("FIND: name {:?}", result);

    let result = db
        .find(&json!({"name":"notsaved11", "leches": 45}))
        .unwrap();
    println!("FIND: name & leches {:?}", result);

    let result = db.find(&json!({"name":"notsaved11"})).unwrap();
    println!("FIND: name {:?}", result);

    let result = db
        .find_id(&json!({"_id":"e7cdef61-d09d-420a-9a3d-e485c056c6aa"}))
        .unwrap();
    println!("FINDONE_ID: name {:?}", result);

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

    let result = db.insert(json!({"name":"record4", "leches": 44})).unwrap();
    println!("INSERT: name & leches {:?}", result);

    db.get();

    // let result = db
    //     .find(&json!({"name":"updatedname", "leches": 333}))
    //     .unwrap();
    // println!("FIND AFTER UPDATE: {:?}", result);
    //println!("UPDATE NAME: name & leches {:?}", result);

    // let result = db
    //     .insert(json!({"name":"jooooosh", "iscool": false}))
    //     .unwrap();
    // println!("INSERT: name & leches {:?}", result);
    // let result3 = db
    //     .find(&json!({"_id":"e94ef3e2-6378-4c63-9d3c-f9751754774f"}))
    //     .unwrap();
    // println!("FIND BY ID {:?}", result3);

    // let result4 = db.insert(json!({"name":"inserted11"})).unwrap();
    // println!("INSERTED{:?}", result4);

    // let result5 = db
    //     .update(json!({"name":"inserted11"}), json!({"name":"inserted33"}))
    //     .unwrap();
    // println!("UPDATED{:?}", result5);
    //    let doc1 = json!({"name":"notsaved1"});
    // let doc = db.insert(doc1).unwrap();
    // let _id = &doc["_id"];
    // println!("ID AFTER INSERT {:?}", _id);
    //db.get();
    //db.persist();
    // let mut router = Router::new();
    // for handler in handlers::get_handlers() {
    //     match handler.method {
    //         handlers::Method::Get => {
    //             println!("Setting up GET method {}", handler.route);
    //             router.get(&handler.route, handler.handler, &handler.route);
    //         }
    //         handlers::Method::Post => {
    //             println!("Setting up POST method {}", handler.route);
    //             router.post(&handler.route, handler.handler, &handler.route);
    //         }
    //     }
    // }
    // let host_addr = dotenv!("HOST_ADDRESS");

    // println!("Server up on http://{}", &host_addr);
    // Iron::new(router).http(&host_addr).unwrap();
}
