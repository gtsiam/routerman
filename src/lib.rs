//! Minimal generic routing framework with hyper integration
//!
//! Just imagine all the documentation. Cause that's all you can do for now since I haven't written
//! it.

pub mod method;
pub mod request;
pub mod response;
pub mod route;
pub mod router;

#[cfg(feature = "json")]
pub mod json;

mod mime;
