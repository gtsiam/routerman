//! Minimal generic routing framework with hyper integration
//!
//! Just imagine all the documentation. Cause that's all you can do for now since I haven't written
//! it.

use hyper::{Body, Request, Response};
use response::DefaultFormatter;
use router::Router;

pub mod method;
pub mod request;
pub mod response;
pub mod route;
pub mod router;

pub type HyperRouter<Fmt = DefaultFormatter> = Router<'static, Request<Body>, Response<Body>, Fmt>;

#[cfg(feature = "json")]
pub mod json;
