use super::{DefaultFormatter, ErrorResponse, Formatter, IntoResponse, ResponsePart};
use hyper::{
    header::{self},
    Body, Response, StatusCode,
};
use std::error::Error as StdError;

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

// impl<const N: usize, K, V, B> ResponsePart<Response<B>> for [(K, V); N]
// where
//     K: TryInto<HeaderName>,
//     V: TryInto<HeaderValue>,
// {
//     fn response_part(self, res: &mut Response<B>) {
//         for (k, v) in self {
//             let k = k.try_into().unwrap();
//             let v = v.try_into().unwrap();
//             res.headers_mut().insert(k, v);
//         }
//     }
// }

impl<Fmt, Err> IntoResponse<Response<Body>, Fmt> for Err
where
    Fmt: Formatter<Response<Body>, Self>,
    Err: ErrorResponse,
{
    fn into_response(self, fmt: Fmt) -> (Response<Body>, Option<Fmt>) {
        (fmt.format_error(self), None)
    }
}

impl<Err> Formatter<Response<Body>, Err> for DefaultFormatter
where
    Err: StdError,
{
    fn format_error(self, err: Err) -> Response<Body> {
        Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .header(header::CONTENT_TYPE, mime::TEXT_PLAIN.as_ref())
            .body(Body::from(err.to_string()))
            .into_response(self)
            .0
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

impl<Fmt> IntoResponse<Response<Body>, Fmt> for &'static str
where
    Fmt: Formatter<Response<Body>, hyper::http::Error>,
{
    fn into_response(self, fmt: Fmt) -> (Response<Body>, Option<Fmt>) {
        Response::builder()
            .header(header::CONTENT_TYPE, mime::TEXT_PLAIN.as_ref())
            .body(Body::from(self))
            .into_response(fmt)
    }
}

impl<Fmt> IntoResponse<Response<Body>, Fmt> for String
where
    Fmt: Formatter<Response<Body>, hyper::http::Error>,
{
    fn into_response(self, fmt: Fmt) -> (Response<Body>, Option<Fmt>) {
        Response::builder()
            .header(header::CONTENT_TYPE, mime::TEXT_PLAIN.as_ref())
            .body(Body::from(self))
            .into_response(fmt)
    }
}
