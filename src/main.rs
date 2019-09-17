use iron::prelude::*;
use router::Router;

mod handlers;

struct Server {
    host: String,
    port: usize,
}

fn main() {
    let server_config = Server {
        host: "localhost".to_string(),
        port: 3000,
    };
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
    let host_addr = [
        server_config.host,
        ":".to_string(),
        server_config.port.to_string(),
    ]
    .concat();

    println!("Server up on http://{}", &host_addr);
    Iron::new(router).http(&host_addr).unwrap();
}
