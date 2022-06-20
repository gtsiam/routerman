use super::{Formatter, IntoResponse};
use hyper::{header, Body, Response};
use serde::Serialize;

pub struct Json<T>(pub T);

pub use serde_json::Error;

impl<T, Fmt> IntoResponse<Fmt, Response<Body>> for Json<T>
where
    T: Serialize,
    Fmt: Formatter<serde_json::Error, Response<Body>>,
{
    fn into_response(self, fmt: Fmt) -> Response<Body> {
        match serde_json::to_vec(&self.0) {
            Ok(content) => Response::builder()
                .header(header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                .body(Body::from(content))
                .into_response(fmt),
            Err(err) => fmt.format_error(err),
        }
    }
}
