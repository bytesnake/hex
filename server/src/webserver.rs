use futures::{Future, Stream, future};
use hyper;
use hyper::Error;
use hyper::server::{Http, Request, Response, Service};
use hyper_staticfile::Static;
use std::path::Path;
use tokio_core::reactor::{Core, Handle};
use tokio_core::net::TcpListener;

type ResponseFuture = Box<Future<Item=Response, Error=Error>>;

struct MainService {
    static_: Static,
}

impl MainService {
    fn new(handle: &Handle) -> MainService {
        MainService {
            static_: Static::new(handle, Path::new("frontend/build/")),
        }
    }
}

impl Service for MainService {
    type Request = Request;
    type Response = Response;
    type Error = Error;
    type Future = ResponseFuture;

    fn call(&self, req: Request) -> Self::Future {
        if req.path() == "/" {
            let res = Response::new()
                .with_status(hyper::StatusCode::MovedPermanently)
                .with_header(hyper::header::Location::new("/index.html"));
            Box::new(future::ok(res))
        } else {
            self.static_.call(req)
        }
    }
}

pub fn create_webserver(host: &str, port: u16) {
    let mut core = Core::new().unwrap();
    let handle = core.handle();

    //let addr = (host, port).parse().unwrap();
    let listener = TcpListener::bind(&format!("{}:{}", host, port).parse().unwrap(), &handle).unwrap();

    let http = Http::new();
    let server = listener.incoming().for_each(|(sock, addr)| {
        let s = MainService::new(&handle);
        http.bind_connection(&handle, sock, addr, s);
        Ok(())
    });

    println!("Web server running on http://{}:{}", host, port);
    core.run(server).unwrap();
}
