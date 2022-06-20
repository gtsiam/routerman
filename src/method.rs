use std::collections::HashMap;

use hyper::{
    header::{self, HeaderValue},
    Body, Method, StatusCode,
};

use crate::{response::IntoResponse, Request};
use crate::{route::Route, Response};

pub struct MethodRouter {
    handlers: HashMap<Method, Route>,
    fallback: MethodFallback,
}

enum MethodFallback {
    Route(Route),
    None { allow_header: HeaderValue },
}

impl MethodRouter {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            fallback: MethodFallback::None {
                allow_header: HeaderValue::from_static(""),
            },
        }
    }

    pub fn set_method(&mut self, method: Method, route: impl Into<Route>) -> &mut Self {
        self.handlers.insert(method, route.into());
        self.update_allow_header();
        self
    }

    #[inline]
    pub fn method(mut self, method: Method, route: impl Into<Route>) -> Self {
        self.set_method(method, route);
        self
    }

    pub fn set_fallback(&mut self, route: impl Into<Route>) -> &mut Self {
        self.fallback = MethodFallback::Route(route.into());
        self
    }

    #[inline]
    pub fn fallback(mut self, route: impl Into<Route>) -> Self {
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
            pub fn $name(route: impl Into<Route>) -> MethodRouter {
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

impl core::ops::BitOr<Self> for MethodRouter {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.merged(rhs)
    }
}

impl core::ops::BitOrAssign<Self> for MethodRouter {
    fn bitor_assign(&mut self, rhs: Self) {
        self.merge(rhs)
    }
}

impl From<MethodRouter> for Route {
    fn from(r: MethodRouter) -> Self {
        (move |req: Request| match r.handlers.get(req.method()) {
            Some(route) => (route.handler())(req),
            None => match &r.fallback {
                MethodFallback::Route(route) => (route.handler())(req),
                MethodFallback::None { allow_header } => {
                    let allow_header = allow_header.clone();
                    Box::pin(async move {
                        Response::builder()
                            .status(StatusCode::METHOD_NOT_ALLOWED)
                            .header(header::ALLOW, allow_header)
                            .body(Body::empty())
                            .into_response()
                    })
                }
            },
        })
        .into()
    }
}

impl Default for MethodRouter {
    fn default() -> Self {
        Self::new()
    }
}
