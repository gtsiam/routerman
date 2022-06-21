use crate::route::{Route, RouteHandler};
use crate::{response::Formatter, router::RouteError};
use hyper::{header::HeaderValue, Body, Method, Request, Response};
use std::{collections::HashMap, future::ready};

pub struct MethodRouter<'h, Req, Res, Fmt> {
    handlers: HashMap<Method, Route<'h, Req, Res, Fmt>>,
    fallback: MethodFallback<'h, Req, Res, Fmt>,
}

enum MethodFallback<'h, Req, Res, Fmt> {
    Route(Route<'h, Req, Res, Fmt>),
    None { allow_header: HeaderValue },
}

impl<'h, Req, Res, Fmt> MethodRouter<'h, Req, Res, Fmt> {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            fallback: MethodFallback::None {
                allow_header: HeaderValue::from_static(""),
            },
        }
    }

    pub fn set_method<H, Args>(&mut self, method: Method, route: H) -> &mut Self
    where
        H: RouteHandler<'h, Req, Res, Fmt, Args>,
    {
        self.handlers.insert(method, route.into_route());
        self.update_allow_header();
        self
    }

    #[inline]
    pub fn method<H, Args>(mut self, method: Method, route: H) -> Self
    where
        H: RouteHandler<'h, Req, Res, Fmt, Args>,
    {
        self.set_method(method, route);
        self
    }

    #[inline]
    pub fn set_fallback(&mut self, route: impl Into<Route<'h, Req, Res, Fmt>>) -> &mut Self {
        self.fallback = MethodFallback::Route(route.into());
        self
    }

    #[inline]
    pub fn fallback(mut self, route: impl Into<Route<'h, Req, Res, Fmt>>) -> Self {
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

            // Header names are possible to encode in http, so this should never panic
            *allow_header = HeaderValue::from_str(&*methods.join(", ")).unwrap();
        }
    }
}

macro_rules! impl_method_helpers {
    ($($name:ident -> $method:ident)*) => {
        $(
            #[inline]
            pub fn $name<'h, H, Req, Res, Fmt, Args>(route: H) -> MethodRouter<'h, Req, Res, Fmt>
            where
                H: RouteHandler<'h, Req, Res, Fmt, Args>
            {
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

impl<'h, Req, Res, Fmt> core::ops::BitOr<Self> for MethodRouter<'h, Req, Res, Fmt> {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Self) -> Self::Output {
        self.merged(rhs)
    }
}

impl<'h, Req, Res, Fmt> core::ops::BitOrAssign<Self> for MethodRouter<'h, Req, Res, Fmt> {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        self.merge(rhs)
    }
}

impl<'h, Fmt, B> RouteHandler<'h, Request<B>, Response<Body>, Fmt, ()>
    for MethodRouter<'h, Request<B>, Response<Body>, Fmt>
where
    Fmt: for<'a> Formatter<Response<Body>, RouteError<'a>>,
    Fmt: Send + Sync + 'h,
    B: 'h,
{
    fn into_route(self) -> Route<'h, Request<B>, Response<Body>, Fmt> {
        Route::new(
            move |req: Request<B>, fmt: Fmt| match self.handlers.get(req.method()) {
                Some(route) => (route.handler_fn())(req, fmt),
                None => match &self.fallback {
                    MethodFallback::Route(route) => (route.handler_fn())(req, fmt),
                    MethodFallback::None { allow_header } => Box::pin(ready(
                        fmt.format_error(RouteError::MethodNotAllowed { allow_header }),
                    )),
                },
            },
        )
    }
}

impl<'h, Req, Res, Fmt> Default for MethodRouter<'h, Req, Res, Fmt> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
