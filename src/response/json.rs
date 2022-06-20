use super::{Formatter, IntoResponse};
use crate::Response;
use hyper::{header, Body};
use serde::Serialize;

pub struct Json<T>(pub T);

pub use serde_json::Error;

impl<T, F> IntoResponse<F> for Json<T>
where
    T: Serialize,
    F: Formatter<serde_json::Error>,
{
    fn into_response(self, fmt: F) -> Response {
        match serde_json::to_vec(&self.0) {
            Ok(content) => Response::builder()
                .header(header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                .body(Body::from(content))
                .into_response(fmt),
            Err(err) => fmt.format_error(err),
        }
    }
}
