use core::fmt::Display;
use std::{
    convert::Infallible,
    future::{Future, Ready},
    net::SocketAddr,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures_util::ready;
use hyper::{
    header::{self},
    server::conn::AddrStream,
    Body, StatusCode, Uri,
};
use matchit::MatchError;
use pin_project::pin_project;
use tower_service::Service;

use crate::{
    request::ext::{InvalidParamEncoding, RemoteAddrExt, RouteParamsExt},
    response::{DefaultFormatter, IntoResponse},
    route::{HandlerFuture, Route},
    Request, Response,
};

pub struct Router<Fmt = DefaultFormatter> {
    formatter: Fmt,
    inner: Arc<RouterImpl<Fmt>>,
}

pub struct RouterImpl<Fmt> {
    inner: matchit::Router<Route<Fmt>>,
    default: Route<Fmt>,
}

impl<Fmt> Router<Fmt>
where
    Fmt: Default + Clone + Send + Sync + 'static,
{
    pub fn builder() -> RouterBuilder<Fmt> {
        RouterBuilder::with_formatter(Fmt::default())
    }
}

impl<Fmt> Router<Fmt>
where
    Fmt: Clone + Send + Sync + 'static,
{
    pub fn with_formatter(fmt: Fmt) -> RouterBuilder<Fmt> {
        RouterBuilder::with_formatter(fmt)
    }
}

pub struct RouterBuilder<Fmt = DefaultFormatter> {
    formatter: Fmt,
    inner: matchit::Router<Route<Fmt>>,
    default: Route<Fmt>,
}

impl<Fmt> RouterBuilder<Fmt>
where
    Fmt: Clone + Send + Sync + 'static,
{
    pub fn with_formatter(formatter: Fmt) -> Self {
        Self {
            inner: matchit::Router::new(),
            default: Route::with_fmt(|_, fmt| async { StatusCode::NOT_FOUND.into_response(fmt) }),
            formatter,
        }
    }

    pub fn route(mut self, path: impl Into<String>, route: impl Into<Route<Fmt>>) -> Self {
        self.inner.insert(path, route.into()).expect("insert route");
        self
    }

    pub fn build(self) -> Router<Fmt> {
        Router {
            inner: Arc::new(RouterImpl {
                inner: self.inner,
                default: self.default,
            }),
            formatter: self.formatter,
        }
    }
}

impl<Fmt: Clone> Service<&AddrStream> for Router<Fmt> {
    type Response = RequestService<Fmt>;
    type Error = Infallible;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, conn: &AddrStream) -> Self::Future {
        let remote_addr = conn.remote_addr();
        std::future::ready(Ok(RequestService {
            remote_addr,
            router: self.inner.clone(),
            formatter: self.formatter.clone(),
        }))
    }
}

pub struct RequestService<Fmt> {
    formatter: Fmt,
    remote_addr: SocketAddr,
    router: Arc<RouterImpl<Fmt>>,
}

impl<Fmt> Service<Request> for RequestService<Fmt>
where
    Fmt: Clone + Send + Sync + 'static,
{
    type Response = Response;
    type Error = Infallible;
    type Future = RequestFuture;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, mut req: Request) -> Self::Future {
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

        req.extensions_mut()
            .insert(RemoteAddrExt::from(self.remote_addr));

        enum MatchResult<'a, Fmt> {
            Route(&'a Route<Fmt>, Option<RouteParamsExt>),
            Redirect(Uri),
            InvalidParamEncoding(InvalidParamEncoding),
        }

        let res = match self.router.inner.at(req.uri().path()) {
            Ok(route) => match route.params.try_into() {
                Ok(params) => MatchResult::Route(route.value, Some(params)),
                Err(err) => MatchResult::InvalidParamEncoding(err),
            },
            Err(MatchError::NotFound) => MatchResult::Route(&self.router.default, None),

            Err(MatchError::ExtraTrailingSlash) => MatchResult::Redirect(replace_path(
                req.uri(),
                req.uri().path().strip_suffix('/').unwrap(),
            )),
            Err(MatchError::MissingTrailingSlash) => MatchResult::Redirect(replace_path(
                req.uri(),
                format_args!("{}/", req.uri().path()),
            )),
        };

        match res {
            MatchResult::Route(route, params) => {
                if let Some(params) = params {
                    req.extensions_mut().insert(params);
                }

                RequestFuture::Route((route.handler())(req, self.formatter.clone()))
            }
            MatchResult::Redirect(uri) => RequestFuture::Response(Some(
                Response::builder()
                    .status(StatusCode::PERMANENT_REDIRECT)
                    .header(header::LOCATION, uri.to_string())
                    .body(Body::empty())
                    .into_response(self.formatter.clone()),
            )),
            MatchResult::InvalidParamEncoding(_err) => RequestFuture::Response(Some(
                StatusCode::BAD_REQUEST.into_response(self.formatter.clone()),
            )),
        }
    }
}

#[pin_project(project = RequestFutureProj)]
pub enum RequestFuture {
    Route(#[pin] HandlerFuture),
    Response(Option<Response>),
}

impl Future for RequestFuture {
    type Output = Result<Response, Infallible>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.as_mut().project() {
            RequestFutureProj::Route(fut) => return Poll::Ready(Ok(ready!(fut.poll(cx)))),
            RequestFutureProj::Response(res @ Some(_)) => Poll::Ready(Ok(res.take().unwrap())),
            RequestFutureProj::Response(None) => panic!("future polled after completion"),
        }
    }
}
