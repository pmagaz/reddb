use dotenv::dotenv;
use iron::prelude::*;
use router::Router;

use dotenv_codegen::dotenv;

mod dstore;
mod handlers;
use dstore::DStore;
#[macro_use]
extern crate quick_error;

fn main() {
    dotenv().ok();
    let mut db = DStore::new(dotenv!("DB_PATH")).unwrap();
    let doc = r#"
        {
            "id": 1,
            "data": {
            }
        }"#;
    let data = doc.to_string();
    db.get();

    db.insert(r#"{"id": "66666666","created_at": "BCH", "data":{}}"#.to_string());
    db.insert(r#"{"id": "77777777","created_at": "BCH", "data":{}}"#.to_string());
    //db.put(r#"{"id": "3333333","createdAt": "BCH", "data":{}}"#.to_string());
    db.get();
    // db.put(r#"{"id": 1,"data": {}}"#.to_string());
    // db.put(r#"{"id": 1,"data": {}}"#.to_string());
    // db.put(r#"{"id": 1,"data": {}}"#.to_string());
    db.persist();
    //db.put("key".to_string(), "value".to_string()).persist();
    //println!("DATA {:?}", data);
    let mut router = Router::new();
    for handler in handlers::get_handlers() {
        match handler.method {
            handlers::Method::Get => {
                println!("Setting up GET method {}", handler.route);
                router.get(&handler.route, handler.handler, &handler.route);
            }
            handlers::Method::Post => {
                println!("Setting up POST method {}", handler.route);
                router.post(&handler.route, handler.handler, &handler.route);
            }
        }
    }
    let host_addr = dotenv!("HOST_ADDRESS");

    println!("Server up on http://{}", &host_addr);
    Iron::new(router).http(&host_addr).unwrap();
}
