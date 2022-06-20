pub mod method;
pub mod request;
pub mod response;
pub mod route;
pub mod router;

pub type Request<B = hyper::Body> = hyper::Request<B>;
pub type Response<B = hyper::Body> = hyper::Response<B>;
