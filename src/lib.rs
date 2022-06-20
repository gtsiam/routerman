use hyper::{Body, Request, Response};
use response::DefaultFormatter;
use router::Router;

pub mod method;
pub mod request;
pub mod response;
pub mod route;
pub mod router;

pub type HyperRouter<Fmt = DefaultFormatter> = Router<'static, Request<Body>, Response<Body>, Fmt>;
