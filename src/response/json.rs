use super::IntoResponse;
use crate::Response;
use hyper::{header, Body, StatusCode};
use serde::Serialize;

pub struct Json<T>(pub T);

impl<T> IntoResponse for Json<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        match serde_json::to_vec(&self.0) {
            Ok(content) => Response::builder()
                .header(header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                .body(Body::from(content))
                .into_response(),
            Err(err) => Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header(header::CONTENT_TYPE, mime::TEXT_PLAIN.as_ref())
                .body(Body::from(err.to_string()))
                .into_response(),
        }
    }
}
