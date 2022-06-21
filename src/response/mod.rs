use std::convert::Infallible;

use hyper::Body;

mod impls;
mod parts;

pub type Response = hyper::Response<Body>;

pub trait Reply<Fmt> {
    fn reply(self, fmt: Fmt) -> Response;
}

pub trait ReplyPart<Fmt> {
    fn reply_part(self, res: Response, fmt: Fmt) -> (Response, Option<Fmt>);
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultFormatter;

impl<T, E, Fmt> Reply<Fmt> for std::result::Result<T, E>
where
    T: Reply<Fmt>,
    E: Reply<Fmt>,
{
    fn reply(self, fmt: Fmt) -> Response {
        match self {
            Ok(res) => res.reply(fmt),
            Err(res) => res.reply(fmt),
        }
    }
}

impl<Fmt> Reply<Fmt> for Infallible {
    fn reply(self, _fmt: Fmt) -> Response {
        match self {}
    }
}

impl<Fmt> Reply<Fmt> for () {
    fn reply(self, _fmt: Fmt) -> Response {
        Response::default()
    }
}

impl<Fmt> ReplyPart<Fmt> for () {
    fn reply_part(self, res: Response, fmt: Fmt) -> (Response, Option<Fmt>) {
        (res, Some(fmt))
    }
}
