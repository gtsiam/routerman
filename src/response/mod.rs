use std::{convert::Infallible, error::Error as StdError};

use hyper::{header, Body, Response, StatusCode};

#[cfg(feature = "json")]
pub mod json;

pub trait IntoResponse<Fmt, Res> {
    fn into_response(self, fmt: Fmt) -> Res;
}

impl<T, E, Fmt, Res> IntoResponse<Fmt, Res> for Result<T, E>
where
    T: IntoResponse<Fmt, Res>,
    E: IntoResponse<Fmt, Res>,
{
    fn into_response(self, fmt: Fmt) -> Res {
        match self {
            Ok(res) => res.into_response(fmt),
            Err(res) => res.into_response(fmt),
        }
    }
}

impl<Fmt, Res> IntoResponse<Fmt, Res> for Infallible {
    fn into_response(self, _fmt: Fmt) -> Res {
        unreachable!()
    }
}

pub trait Formatter<Err, Res> {
    fn format_error(self, err: Err) -> Res;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultFormatter;

impl<Err> Formatter<Err, Response<Body>> for DefaultFormatter
where
    Err: StdError,
{
    fn format_error(self, err: Err) -> Response<Body> {
        Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .header(header::CONTENT_TYPE, mime::TEXT_PLAIN.as_ref())
            .body(Body::from(err.to_string()))
            .into_response(self)
    }
}

impl<Fmt, B> IntoResponse<Fmt, Response<B>> for Response<B> {
    fn into_response(self, _fmt: Fmt) -> Response<B> {
        self
    }
}

impl<Fmt> IntoResponse<Fmt, Response<Body>> for () {
    fn into_response(self, _fmt: Fmt) -> Response<Body> {
        Response::new(Body::empty())
    }
}

impl<Fmt> IntoResponse<Fmt, Response<Body>> for StatusCode {
    fn into_response(self, fmt: Fmt) -> Response<Body> {
        let mut res = ().into_response(fmt);
        *res.status_mut() = self;
        res
    }
}

impl<Fmt> IntoResponse<Fmt, Response<Body>> for hyper::http::Error {
    fn into_response(self, _fmt: Fmt) -> Response<Body> {
        let mut res = Response::new(Body::from(self.to_string()));
        *res.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
        res
    }
}

impl<Fmt> IntoResponse<Fmt, Response<Body>> for &'static str {
    fn into_response(self, fmt: Fmt) -> Response<Body> {
        Response::builder()
            .header(header::CONTENT_TYPE, mime::TEXT_PLAIN.as_ref())
            .body(Body::from(self))
            .into_response(fmt)
    }
}

impl<Fmt> IntoResponse<Fmt, Response<Body>> for String {
    fn into_response(self, fmt: Fmt) -> Response<Body> {
        Response::builder()
            .header(header::CONTENT_TYPE, mime::TEXT_PLAIN.as_ref())
            .body(Body::from(self))
            .into_response(fmt)
    }
}
