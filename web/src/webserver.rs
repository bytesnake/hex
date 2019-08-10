//! The HTTP implementation serves the frontend

use futures::{Async::*, Future, Poll, future};
use http::response::Builder as ResponseBuilder;
use http::{Request, Response, StatusCode, header};
use hyper::{Body, service::Service, header::{HeaderValue, CONTENT_TYPE}};
use hyper_staticfile::{Static, StaticFuture};
use std::path::Path;
use std::io::Error;
use std::net::SocketAddr;
use std::path::PathBuf;

/// Future returned from `MainService`.
enum MainFuture {
    Root,
    Static((StaticFuture<Body>, PathBuf)),
}

impl Future for MainFuture {
    type Item = Response<Body>;
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match *self {
            MainFuture::Root => {
                let res = ResponseBuilder::new()
                    .status(StatusCode::MOVED_PERMANENTLY)
                    .header(header::LOCATION, "/index.html")
                    .body(Body::empty())
                    .expect("unable to build response");
                Ok(Ready(res))
            },
            MainFuture::Static((ref mut future, ref path)) => {
                let mut x = try_ready!(future.poll());

                if let Some(ext) = path.extension() {
                    if let Some("wasm") = ext.to_str() {
                        x.headers_mut().insert(CONTENT_TYPE, HeaderValue::from_static("application/wasm"));
                    }
                }

                Ok(Ready(x))
            }
        }
    }
}

/// The service should just offer all fields in a single directory
struct MainService {
    static_: Static,
    download: Static
}

impl MainService {
    /// Create a new service
    fn new(path: &Path, data_path: &Path) -> MainService {
        MainService {
            static_: Static::new(path),
            download: Static::new(data_path.parent().unwrap())
        }
    }
}

impl Service for MainService {
    type ReqBody = Body;
    type ResBody = Body;
    type Error = Error;
    type Future = MainFuture;

    fn call(&mut self, req: Request<Body>) -> MainFuture {
        let path = PathBuf::from(req.uri().path());
        //println!("Path: {:?}", path);

        if req.uri().path() == "/" {
            MainFuture::Root
        } else {
            MainFuture::Static((self.static_.serve(req), path))
        }
    }
}

/// Create the webserver
///
/// * `addr` - Listen to this address
/// * `path` - Serve this directory
/// * `data_path` - Serve the data from this directory
pub fn create_webserver(addr: SocketAddr, path: PathBuf, data_path: PathBuf) {
    let server = hyper::Server::bind(&addr)
        .serve(move || future::ok::<_, Error>(MainService::new(&path, &data_path)))
        .map_err(|e| eprintln!("server error: {}", e));

    println!("Web server running on http://{}", addr);
    hyper::rt::run(server);
}
