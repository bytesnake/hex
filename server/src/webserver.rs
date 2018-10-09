//! The HTTP implementation serves the frontend
use futures::{Future, Stream, future};
use hyper;
use hyper::Error;
use hyper::server::{Http, Request, Response, Service};
use hyper_staticfile::Static;
use std::path::Path;
use tokio_core::reactor::{Core, Handle};
use tokio_core::net::TcpListener;
use std::net::SocketAddr;

type ResponseFuture = Box<Future<Item=Response, Error=Error>>;

/// The service should just offer all fields in a single directory
struct MainService {
    static_: Static,
    download: Static
}

impl MainService {
    /// Create a new service
    fn new(handle: &Handle, path: &Path, data_path: &Path) -> MainService {
        MainService {
            static_: Static::new(&handle.clone(), path),
            download: Static::new(&handle, data_path.parent().unwrap())
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

/// Create the webserver
///
/// * `addr` - Listen to this address
/// * `path` - Serve this directory
/// * `data_path` - Serve the data from this directory
pub fn create_webserver(addr: SocketAddr, path: &Path, data_path: &Path) {
    let mut core = Core::new().unwrap();
    let handle = core.handle();

    //let addr = (host, port).parse().unwrap();
    let listener = TcpListener::bind(&addr, &handle).unwrap();

    let http = Http::new();
    let server = listener.incoming().for_each(|(sock, addr)| {
        let s = MainService::new(&handle, path, data_path);
        http.bind_connection(&handle, sock, addr, s);
        Ok(())
    });

    println!("Web server running on http://{}", addr);
    core.run(server).unwrap();
}
