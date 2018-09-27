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
    download: Static
}

impl MainService {
    fn new(handle: &Handle, path: &str, data_path: &str) -> MainService {
        MainService {
            static_: Static::new(&handle.clone(), Path::new(path)),
            download: Static::new(&handle, Path::new(data_path).parent().unwrap())
        }
    }
}

impl Service for MainService {
    type Request = Request;
    type Response = Response;
    type Error = Error;
    type Future = ResponseFuture;

    fn call(&self, req: Request) -> Self::Future {
        println!("Path: {}", req.path());
        /*if req.path().starts_with("/data/") {
            println!("Starts with!");

            self.download.call(req)
        } else {*/
            self.static_.call(req)
        //}
    }
}

pub fn create_webserver(host: &str, port: u16, path: &str, data_path: &str) {
    let mut core = Core::new().unwrap();
    let handle = core.handle();

    //let addr = (host, port).parse().unwrap();
    let listener = TcpListener::bind(&format!("{}:{}", host, port).parse().unwrap(), &handle).unwrap();

    let http = Http::new();
    let server = listener.incoming().for_each(|(sock, addr)| {
        let s = MainService::new(&handle, path, data_path);
        http.bind_connection(&handle, sock, addr, s);
        Ok(())
    });

    println!("Web server running on http://{}:{}", host, port);
    core.run(server).unwrap();
}
