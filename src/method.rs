use std::collections::HashMap;

use hyper::{
    header::{self, HeaderValue},
    Body, Method, StatusCode,
};

use crate::{response::IntoResponse, Request};
use crate::{route::Route, Response};

pub struct MethodRouter<'h, Fmt, B> {
    handlers: HashMap<Method, Route<'h, Fmt, B>>,
    fallback: MethodFallback<'h, Fmt, B>,
}

enum MethodFallback<'h, Fmt, B> {
    Route(Route<'h, Fmt, B>),
    None { allow_header: HeaderValue },
}

impl<'h, Fmt, B> MethodRouter<'h, Fmt, B> {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            fallback: MethodFallback::None {
                allow_header: HeaderValue::from_static(""),
            },
        }
    }

    pub fn set_method(&mut self, method: Method, route: impl Into<Route<'h, Fmt, B>>) -> &mut Self {
        self.handlers.insert(method, route.into());
        self.update_allow_header();
        self
    }

    #[inline]
    pub fn method(mut self, method: Method, route: impl Into<Route<'h, Fmt, B>>) -> Self {
        self.set_method(method, route);
        self
    }

    pub fn set_fallback(&mut self, route: impl Into<Route<'h, Fmt, B>>) -> &mut Self {
        self.fallback = MethodFallback::Route(route.into());
        self
    }

    #[inline]
    pub fn fallback(mut self, route: impl Into<Route<'h, Fmt, B>>) -> Self {
        self.set_fallback(route);
        self
    }

    pub fn merge(&mut self, other: Self) {
        self.handlers.extend(other.handlers.into_iter());
        match (&self.fallback, other.fallback) {
            (MethodFallback::Route(_), MethodFallback::Route(_)) => {
                panic!("Cannot merge two method routers with fallback routes")
            }
            (_, MethodFallback::Route(route)) => self.fallback = MethodFallback::Route(route),
            _ => (),
        };

        self.update_allow_header();
    }

    #[inline]
    pub fn merged(mut self, other: Self) -> Self {
        self.merge(other);
        self
    }

    fn update_allow_header(&mut self) {
        if let MethodFallback::None { allow_header } = &mut self.fallback {
            let mut methods = self
                .handlers
                .keys()
                .map(|method| format!("{}", method))
                .collect::<Vec<_>>();
            methods.sort();

            // Header names are possible to encode in http, so this should not panic
            *allow_header = HeaderValue::from_str(&*methods.join(", ")).unwrap();
        }
    }
}

macro_rules! impl_method_helpers {
    ($($name:ident -> $method:ident)*) => {
        $(
            #[inline]
            pub fn $name<'h, Fmt, B>(route: impl Into<Route<'h, Fmt, B>>) -> MethodRouter<'h, Fmt, B> {
                MethodRouter::new().method(Method::$method, route)
            }
        )*
    };
}

impl_method_helpers! {
    get -> GET
    post -> POST
    put -> PUT
    delete -> DELETE
    head -> HEAD
    options -> OPTIONS
    connect -> CONNECT
    patch -> PATCH
    trace -> TRACE
}

impl<'h, Fmt, B> core::ops::BitOr<Self> for MethodRouter<'h, Fmt, B> {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.merged(rhs)
    }
}

impl<'h, Fmt, B> core::ops::BitOrAssign<Self> for MethodRouter<'h, Fmt, B> {
    fn bitor_assign(&mut self, rhs: Self) {
        self.merge(rhs)
    }
}

impl<'h, Fmt, B> From<MethodRouter<'h, Fmt, B>> for Route<'h, Fmt, B>
where
    Fmt: Clone + Send + Sync + 'h,
    B: 'h,
{
    fn from(r: MethodRouter<'h, Fmt, B>) -> Self {
        Route::with_fmt(
            move |req: Request<B>, fmt: Fmt| match r.handlers.get(req.method()) {
                Some(route) => (route.handler())(req, fmt),
                None => match &r.fallback {
                    MethodFallback::Route(route) => (route.handler())(req, fmt),
                    MethodFallback::None { allow_header } => {
                        let allow_header = allow_header.clone();
                        Box::pin(async move {
                            Response::builder()
                                .status(StatusCode::METHOD_NOT_ALLOWED)
                                .header(header::ALLOW, allow_header)
                                .body(Body::empty())
                                .into_response(fmt)
                        })
                    }
                },
            },
        )
    }
}

impl<'h, Fmt, B> Default for MethodRouter<'h, Fmt, B> {
    fn default() -> Self {
        Self::new()
    }
}
