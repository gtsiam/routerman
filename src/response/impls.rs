use crate::{mime::TEXT_PLAIN, router::RouteError, try_respond};

use super::{DefaultFormatter, ErrorResponse, Formatter, IntoResponse, ResponsePart};
use hyper::{
    header::{self, HeaderValue, InvalidHeaderName, InvalidHeaderValue},
    Body, Response, StatusCode,
};

impl<Fmt, B> ResponsePart<Response<B>, Fmt> for StatusCode {
    fn response_part(self, mut res: Response<B>, fmt: Fmt) -> (Response<B>, Option<Fmt>) {
        *res.status_mut() = self;
        (res, Some(fmt))
    }
}

impl<Fmt> ResponsePart<Response<Body>, Fmt> for Body {
    fn response_part(self, mut res: Response<Body>, fmt: Fmt) -> (Response<Body>, Option<Fmt>) {
        *res.body_mut() = self;
        (res, Some(fmt))
    }
}

impl<const N: usize, Fmt, B, K, V> ResponsePart<Response<B>, Fmt> for [(K, V); N]
where
    K: TryInto<header::HeaderName>,
    V: TryInto<header::HeaderValue>,
    Fmt: Formatter<Response<B>, K::Error> + Formatter<Response<B>, V::Error>,
{
    fn response_part(self, mut res: Response<B>, fmt: Fmt) -> (Response<B>, Option<Fmt>) {
        for (k, v) in self {
            let k = try_respond!(k.try_into(), fmt);
            let v = try_respond!(v.try_into(), fmt);
            res.headers_mut().insert(k, v);
        }
        (res, Some(fmt))
    }
}

impl<Fmt, Res, Err> IntoResponse<Res, Fmt> for Err
where
    Fmt: Formatter<Res, Self>,
    Err: ErrorResponse,
{
    fn into_response(self, fmt: Fmt) -> (Res, Option<Fmt>) {
        (fmt.format_error(self), None)
    }
}

impl<Fmt, B> IntoResponse<Response<B>, Fmt> for Response<B> {
    fn into_response(self, fmt: Fmt) -> (Response<B>, Option<Fmt>) {
        (self, Some(fmt))
    }
}

impl<Fmt> IntoResponse<Response<Body>, Fmt> for () {
    fn into_response(self, fmt: Fmt) -> (Response<Body>, Option<Fmt>) {
        (Response::new(Body::empty()), Some(fmt))
    }
}

impl<Fmt> IntoResponse<Response<Body>, Fmt> for StatusCode {
    fn into_response(self, fmt: Fmt) -> (Response<Body>, Option<Fmt>) {
        (self, ()).into_response(fmt)
    }
}

impl ErrorResponse for hyper::http::Error {}

impl Formatter<Response<Body>, hyper::http::Error> for DefaultFormatter {
    fn format_error(self, err: hyper::http::Error) -> Response<Body> {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            [(header::CONTENT_TYPE, TEXT_PLAIN.as_str())],
            err.to_string(),
        )
            .into_response(self)
            .0
    }
}

// ---

impl Formatter<Response<Body>, InvalidHeaderName> for DefaultFormatter {
    fn format_error(self, err: InvalidHeaderName) -> Response<Body> {
        (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
            .into_response(self)
            .0
    }
}

impl Formatter<Response<Body>, InvalidHeaderValue> for DefaultFormatter {
    fn format_error(self, err: InvalidHeaderValue) -> Response<Body> {
        (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
            .into_response(self)
            .0
    }
}

impl Formatter<Response<Body>, RouteError<'_>> for DefaultFormatter {
    fn format_error(self, err: RouteError) -> Response<Body> {
        match err {
            RouteError::NotFound => (StatusCode::NOT_FOUND, ()).into_response(self).0,
            RouteError::Path(_err) => (StatusCode::BAD_REQUEST, ()).into_response(self).0,
            RouteError::MethodNotAllowed {
                allow_header: allow_headers,
            } => {
                (
                    StatusCode::METHOD_NOT_ALLOWED,
                    [(header::ALLOW, allow_headers)],
                    (),
                )
                    .into_response(self)
                    .0
            }
            RouteError::Expected(uri) => {
                (
                    StatusCode::PERMANENT_REDIRECT,
                    [(header::LOCATION, uri.to_string())],
                    (),
                )
                    .into_response(self)
                    .0
            }
        }
    }
}

// ---

impl<Fmt> IntoResponse<Response<Body>, Fmt> for &'static str {
    fn into_response(self, fmt: Fmt) -> (Response<Body>, Option<Fmt>) {
        let mut res = Response::new(Body::from(self));
        res.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static(TEXT_PLAIN.as_str()),
        );
        (res, Some(fmt))
    }
}

impl<Fmt> IntoResponse<Response<Body>, Fmt> for String {
    fn into_response(self, fmt: Fmt) -> (Response<Body>, Option<Fmt>) {
        let mut res = Response::new(Body::from(self));
        res.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static(TEXT_PLAIN.as_str()),
        );
        (res, Some(fmt))
    }
}
