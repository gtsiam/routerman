pub mod method;
pub mod request;
pub mod response;
pub mod route;
pub mod router;

pub type HyperRouter =
    router::Router<'static, hyper::Request<hyper::Body>, hyper::Response<hyper::Body>>;
