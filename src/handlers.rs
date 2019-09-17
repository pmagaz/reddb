use iron::prelude::*;
use iron::status;

type HandlerFn = fn(&mut Request) -> IronResult<Response>;

pub enum Method {
    Post,
    Get,
}

pub struct Handler {
    pub method: Method,
    pub route: String,
    pub handler: HandlerFn,
}

pub fn get_handlers() -> Vec<Handler> {
    let register = Handler {
        method: Method::Post,
        route: "/webpush/register".to_string(),
        handler: |req: &mut Request| -> IronResult<Response> {
            let response = match req.get::<bodyparser::Json>() {
                // println!("Parsed body:\n{:?}", json_body);
                Ok(Some(_json_body)) => Response::with(status::Ok),
                Ok(None) => Response::with(status::NotAcceptable),
                Err(_err) => Response::with(status::NotAcceptable),
            };
            Ok(response)
        },
    };

    let send_push = Handler {
        method: Method::Get,
        route: "/webpush/sendpush".to_string(),
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
