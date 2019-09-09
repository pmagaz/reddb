use iron::prelude::*;
use router::Router;

mod handlers;

fn main() {
    let mut router = Router::new();
    for handler in handlers::get_handlers() {
        match handler.method {
            handlers::Method::Get => {
                println!("Setting up POST method {}", handler.url);
                router.get("/", handler.handler, "root");
            }
            handlers::Method::Post => {
                println!("Setting up POST method {}", handler.url);
                router.get("/", handler.handler, "root");
            }
        }
    }

    println!("Serving on http://localhost:3000...");
    Iron::new(router).http("localhost:3000").unwrap();
}
