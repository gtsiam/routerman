use hyper::{header, Body, StatusCode};

use crate::Response;

#[cfg(feature = "json")]
pub mod json;

pub trait IntoResponse<F> {
    fn into_response(self, fmt: F) -> Response;
}

pub trait Formatter<E> {
    fn format_error(self, err: E) -> Response;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultFormatter;

impl<E: std::error::Error> Formatter<E> for DefaultFormatter {
    fn format_error(self, err: E) -> Response {
        Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .header(header::CONTENT_TYPE, mime::TEXT_PLAIN.as_ref())
            .body(Body::from(err.to_string()))
            .into_response(self)
    }
}

impl<F> IntoResponse<F> for Response {
    fn into_response(self, _fmt: F) -> Response {
        self
    }
}

impl<F> IntoResponse<F> for () {
    fn into_response(self, _fmt: F) -> Response {
        Response::new(Body::empty())
    }
}

impl<F> IntoResponse<F> for StatusCode {
    fn into_response(self, fmt: F) -> Response {
        let mut res = ().into_response(fmt);
        *res.status_mut() = self;
        res
    }
}

impl<F> IntoResponse<F> for Body {
    fn into_response(self, _fmt: F) -> Response {
        Response::new(self)
    }
}

impl<T, E, F> IntoResponse<F> for Result<T, E>
where
    T: IntoResponse<F>,
    E: IntoResponse<F>,
{
    fn into_response(self, fmt: F) -> Response {
        match self {
            Ok(res) => res.into_response(fmt),
            Err(res) => res.into_response(fmt),
        }
    }
}

impl<F> IntoResponse<F> for hyper::http::Error {
    fn into_response(self, fmt: F) -> Response {
        let mut res = Body::from(self.to_string()).into_response(fmt);
        *res.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
        res
    }
}

impl<F> IntoResponse<F> for &'static str {
    fn into_response(self, fmt: F) -> Response {
        Response::builder()
            .header(header::CONTENT_TYPE, mime::TEXT_PLAIN.as_ref())
            .body(Body::from(self))
            .into_response(fmt)
    }
}

impl<F> IntoResponse<F> for String {
    fn into_response(self, fmt: F) -> Response {
        Response::builder()
            .header(header::CONTENT_TYPE, mime::TEXT_PLAIN.as_ref())
            .body(Body::from(self))
            .into_response(fmt)
    }
}
