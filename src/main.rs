use dotenv::dotenv;
use iron::prelude::*;
use router::Router;

use dotenv_codegen::dotenv;

mod handlers;

fn main() {
    dotenv().ok();
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
