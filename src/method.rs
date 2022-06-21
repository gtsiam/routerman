use crate::{
    request::Request,
    response::Reply,
    route::{Route, RouteHandler},
};
use hyper::{header::HeaderValue, Method};
use std::{collections::HashMap, future::ready};

pub struct MethodRouter<Fmt> {
    handlers: HashMap<Method, Route<Fmt>>,
    fallback: MethodFallback<Fmt>,
}

enum MethodFallback<Fmt> {
    Route(Route<Fmt>),
    None { allow_header: HeaderValue },
}

impl<Fmt> MethodRouter<Fmt> {
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
        H: RouteHandler<Fmt, Args>,
    {
        self.handlers.insert(method, route.into_route());
        self.update_allow_header();
        self
    }

    #[inline]
    pub fn method<H, Args>(mut self, method: Method, route: H) -> Self
    where
        H: RouteHandler<Fmt, Args>,
    {
        self.set_method(method, route);
        self
    }

    #[inline]
    pub fn set_fallback(&mut self, route: impl Into<Route<Fmt>>) -> &mut Self {
        self.fallback = MethodFallback::Route(route.into());
        self
    }

    #[inline]
    pub fn fallback(mut self, route: impl Into<Route<Fmt>>) -> Self {
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
            pub fn $name<H, Fmt, Args>(route: H) -> MethodRouter< Fmt>
            where
                H: RouteHandler< Fmt, Args>
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

impl<Fmt> core::ops::BitOr<Self> for MethodRouter<Fmt> {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Self) -> Self::Output {
        self.merged(rhs)
    }
}

impl<Fmt> core::ops::BitOrAssign<Self> for MethodRouter<Fmt> {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        self.merge(rhs)
    }
}

pub struct MethodNotAllowed<'a> {
    pub allow_header: &'a HeaderValue,
}

impl<Fmt> RouteHandler<Fmt, ()> for MethodRouter<Fmt>
where
    Fmt: Send + Sync + 'static,
    for<'a> MethodNotAllowed<'a>: Reply<Fmt>,
{
    fn into_route(self) -> Route<Fmt> {
        Route::new(
            move |req: Request, fmt: Fmt| match self.handlers.get(req.method()) {
                Some(route) => (route.handler_fn())(req, fmt),
                None => match &self.fallback {
                    MethodFallback::Route(route) => (route.handler_fn())(req, fmt),
                    MethodFallback::None { allow_header } => {
                        Box::pin(ready(MethodNotAllowed { allow_header }.reply(fmt)))
                    }
                },
            },
        )
    }
}

impl<Fmt> Default for MethodRouter<Fmt> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
