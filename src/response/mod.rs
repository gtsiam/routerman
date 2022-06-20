use hyper::{header, Body, StatusCode};

use crate::Response;

#[cfg(feature = "json")]
pub mod json;

pub trait IntoResponse {
    fn into_response(self) -> Response;
}

impl IntoResponse for Response {
    fn into_response(self) -> Response {
        self
    }
}

impl IntoResponse for () {
    fn into_response(self) -> Response {
        Response::new(Body::empty())
    }
}

impl IntoResponse for StatusCode {
    fn into_response(self) -> Response {
        let mut res = ().into_response();
        *res.status_mut() = self;
        res
    }
}

impl IntoResponse for Body {
    fn into_response(self) -> Response {
        Response::new(self)
    }
}

impl<T, E> IntoResponse for Result<T, E>
where
    T: IntoResponse,
    E: IntoResponse,
{
    fn into_response(self) -> Response {
        match self {
            Ok(res) => res.into_response(),
            Err(res) => res.into_response(),
        }
    }
}

impl IntoResponse for hyper::http::Error {
    fn into_response(self) -> Response {
        let mut res = Body::from(self.to_string()).into_response();
        *res.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
        res
    }
}

impl IntoResponse for &'static str {
    fn into_response(self) -> Response {
        Response::builder()
            .header(header::CONTENT_TYPE, mime::TEXT_PLAIN.as_ref())
            .body(Body::from(self))
            .into_response()
    }
}

impl IntoResponse for String {
    fn into_response(self) -> Response {
        Response::builder()
            .header(header::CONTENT_TYPE, mime::TEXT_PLAIN.as_ref())
            .body(Body::from(self))
            .into_response()
    }
}
