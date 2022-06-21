use std::{convert::Infallible, error::Error as StdError};

use hyper::Body;
mod impls;
mod parts;

// pub struct Response {
//     inner: hyper::Response<Body>,
// }

pub trait IntoResponse<Res, Fmt> {
    fn into_response(self, fmt: Fmt) -> (Res, Option<Fmt>);
}

pub trait ResponsePart<Res, Fmt> {
    fn response_part(self, res: Res, fmt: Fmt) -> (Res, Option<Fmt>);
}

pub trait Formatter<Res, Err> {
    fn format_error(self, err: Err) -> Res;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultFormatter;

pub trait ErrorResponse: StdError {}

impl<T, E, Res, Fmt> IntoResponse<Res, Fmt> for std::result::Result<T, E>
where
    T: IntoResponse<Res, Fmt>,
    E: IntoResponse<Res, Fmt>,
{
    fn into_response(self, fmt: Fmt) -> (Res, Option<Fmt>) {
        match self {
            Ok(res) => res.into_response(fmt),
            Err(res) => res.into_response(fmt),
        }
    }
}

impl<Res, Fmt> Formatter<Res, Infallible> for Fmt {
    fn format_error(self, err: Infallible) -> Res {
        match err {}
    }
}

impl<Res, Fmt> IntoResponse<Res, Fmt> for Infallible {
    fn into_response(self, _fmt: Fmt) -> (Res, Option<Fmt>) {
        match self {}
    }
}

#[macro_export]
macro_rules! respond {
    ($into_response:expr, $fmt:expr) => {
        match $into_response.into_response($fmt) {
            (res, Some(fmt)) => (res, fmt),
            (res, None) => return (res, None),
        }
    };
    ($into_response:expr, $res:expr, $fmt:expr) => {
        match $into_response.response_part($res, $fmt) {
            (res, Some(fmt)) => (res, fmt),
            (res, None) => return (res, None),
        }
    };
}

#[macro_export]
macro_rules! try_respond {
    ($result:expr, $formatter:expr) => {
        match $result {
            Ok(val) => val,
            Err(err) => return ($formatter.format_error(err), None),
        }
    };
}
