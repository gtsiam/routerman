use super::{DefaultFormatter, Reply, ReplyPart, Response};
use crate::{
    method::MethodNotAllowed,
    mime::TEXT_PLAIN,
    router::{RouteError, RouteErrorKind},
};
use hyper::{
    body::Bytes,
    header::{self, HeaderValue, InvalidHeaderName, InvalidHeaderValue},
    http::{
        method::InvalidMethod,
        status::InvalidStatusCode,
        uri::{InvalidUri, InvalidUriParts},
    },
    Body, StatusCode, Uri,
};
use std::{borrow::Cow, fmt::Display};

// Convinence macro for repeating reply implementations
macro_rules! impl_reply {
    ($(impl$(<$($gen_arg:ident),*>)? Reply<$($gen_ty:ty),*> for $items:tt),*) => {
        $(impl_reply!(@impl ($($($gen_arg),*)?) ($($gen_ty),*) $items);)*
    };

    (@impl $gen_arg:tt $gen_ty:tt { $($ty:tt => $spec:tt),* $(,)? }) => {
        $( impl_reply!(@impl $gen_arg $gen_ty $ty => $spec); )*
    };
    (@impl $gen_arg:tt $gen_ty:tt $(($($ty:ty),*) => $spec:tt),* $(,)?) => {
        $( $( impl_reply!(@impl $gen_arg $gen_ty $ty => $spec); )* )*
    };

    (@impl
        ($($gen_arg:ident),*) ($fmt_ty:ty)
        $ty:ty => [ from $from:ty ]
    ) => {
        impl<$($gen_arg),*> Reply<$fmt_ty> for $ty {
            fn reply(self, fmt: $fmt_ty) -> Response {
                <$from>::from(self).reply(fmt)
            }
        }
    };

    (@impl
        ($($gen_arg:ident),*) ($fmt_ty:ty)
        $ty:ty => [ from_parts ]
    ) => {
        impl<$($gen_arg),*> Reply<$fmt_ty> for $ty
        where
            Self: ReplyPart<$fmt_ty>,
        {
            fn reply(self, fmt: $fmt_ty) -> Response {
                (self,).reply(fmt)
            }
        }
    };
}

impl Reply<DefaultFormatter> for hyper::http::Error {
    fn reply(self, fmt: DefaultFormatter) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).reply(fmt)
    }
}

impl_reply!(
    impl Reply<DefaultFormatter> for {
        (
            InvalidHeaderName, InvalidHeaderValue,
            InvalidMethod, InvalidStatusCode,
            InvalidUri, InvalidUriParts
        ) => [from hyper::http::Error]
    }
);

impl Reply<DefaultFormatter> for RouteError {
    fn reply(self, fmt: DefaultFormatter) -> Response {
        // Replace the path portion of a uri
        fn replace_path(uri: &Uri, path: impl Display) -> Uri {
            let mut parts = uri.to_owned().into_parts();
            parts.path_and_query = parts.path_and_query.map(|pq| {
                match pq.query() {
                    Some(query) => format!("{}?{}", path, query),
                    None => format!("{}", path),
                }
                .parse()
                .unwrap()
            });
            Uri::from_parts(parts).unwrap()
        }

        let Self { request: req, kind } = self;
        match kind {
            RouteErrorKind::NotFound => (StatusCode::NOT_FOUND,).reply(fmt),
            RouteErrorKind::ExtraTrailingSlash => (
                StatusCode::PERMANENT_REDIRECT,
                [(
                    header::LOCATION,
                    replace_path(req.uri(), req.uri().path().strip_suffix('/').unwrap())
                        .to_string(),
                )],
            )
                .reply(fmt),
            RouteErrorKind::MissingTrailingSlash => (
                StatusCode::PERMANENT_REDIRECT,
                [(
                    header::LOCATION,
                    replace_path(req.uri(), format_args!("{}/", req.uri().path())).to_string(),
                )],
            )
                .reply(fmt),
            RouteErrorKind::Param(_) => (StatusCode::BAD_REQUEST,).reply(fmt),
        }
    }
}

impl Reply<DefaultFormatter> for MethodNotAllowed<'_> {
    fn reply(self, fmt: DefaultFormatter) -> Response {
        (
            StatusCode::METHOD_NOT_ALLOWED,
            [(header::ALLOW, self.allow_header)],
        )
            .reply(fmt)
    }
}

impl<Fmt> Reply<Fmt> for Response {
    fn reply(self, _fmt: Fmt) -> Response {
        self
    }
}

impl<const N: usize, Fmt, K, V> ReplyPart<Fmt> for [(K, V); N]
where
    K: TryInto<header::HeaderName>,
    V: TryInto<header::HeaderValue>,

    K::Error: Reply<Fmt>,
    V::Error: Reply<Fmt>,
{
    fn reply_part(self, mut res: Response, fmt: Fmt) -> (Response, Option<Fmt>) {
        for (k, v) in self {
            let k = match k.try_into() {
                Ok(k) => k,
                Err(err) => return (err.reply(fmt), None),
            };
            let v = match v.try_into() {
                Ok(v) => v,
                Err(err) => return (err.reply(fmt), None),
            };
            res.headers_mut().insert(k, v);
        }
        (res, Some(fmt))
    }
}

impl<Fmt> ReplyPart<Fmt> for StatusCode {
    fn reply_part(self, mut res: Response, fmt: Fmt) -> (Response, Option<Fmt>) {
        *res.status_mut() = self;
        (res, Some(fmt))
    }
}

macro_rules! impl_reply_parts {
    ($($ty:ty => |$self:ident, $res:ident, $fmt:ident| $impl_part:block)*) => {
        $(impl<Fmt> ReplyPart<Fmt> for $ty {
            fn reply_part(
                $self,
                mut $res: Response,
                $fmt: Fmt
            ) -> (Response, Option<Fmt>) {
                $impl_part
            }
        })*
    };
}

impl_reply_parts! {
    Body => |self, res, fmt| {
        *res.body_mut() = self;
        (res, Some(fmt))
    }
    &'static [u8] => |self, res, fmt| {
        *res.body_mut() = Body::from(self);
        (res, Some(fmt))
    }
    &'static str => |self, res, fmt| {
        *res.body_mut() = Body::from(self);
        res.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static(TEXT_PLAIN.as_str()),
        );
        (res, Some(fmt))
    }
    Bytes => |self, res, fmt| {
        *res.body_mut() = Body::from(self);
        (res, Some(fmt))
    }
    Cow<'static, [u8]> => |self, res, fmt| {
        *res.body_mut() = Body::from(self);
        (res, Some(fmt))
    }
    Cow<'static, str> => |self, res, fmt| {
        *res.body_mut() = Body::from(self);
        res.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static(TEXT_PLAIN.as_str()),
        );
        (res, Some(fmt))
    }
    String => |self, res, fmt| {
        *res.body_mut() = Body::from(self);
        res.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static(TEXT_PLAIN.as_str()),
        );
        (res, Some(fmt))
    }
    Vec<u8> => |self, res, fmt| {
        *res.body_mut() = Body::from(self);
        (res, Some(fmt))
    }
}

impl_reply! {
    impl<Fmt> Reply<Fmt> for {
        (
            StatusCode,
            Body, &'static [u8], &'static str, Bytes,
            Cow<'static, [u8]>, Cow<'static, str>, String, Vec<u8>
        ) => [from_parts]
    }
}
