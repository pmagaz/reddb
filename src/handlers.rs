use iron::prelude::*;
use iron::status;
//use router::Router;

type HandlerFn = fn(&mut Request) -> IronResult<Response>;

pub enum Method {
    Post,
    Get,
}

pub struct Handler {
    pub method: Method,
    pub url: String,
    pub handler: HandlerFn,
}

pub fn get_handlers() -> Vec<Handler> {
    let register = Handler {
        method: Method::Post,
        url: "/webpush/register".to_string(),
        handler: |_req: &mut Request| -> IronResult<Response> {
            let mut response = Response::new();
            response.set_mut(status::Ok);
            println!("New request");
            Ok(response)
        },
    };

    let send_push = Handler {
        method: Method::Get,
        url: "/webpush/sendpush".to_string(),
        handler: |_req: &mut Request| -> IronResult<Response> {
            let mut response = Response::new();
            response.set_mut(status::Ok);
            println!("New request");
            Ok(response)
        },
    };

    let handlers: Vec<Handler> = vec![register, send_push];
    handlers
}
