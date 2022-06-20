pub(crate) mod ext;
pub(crate) mod params;

pub mod extract;
use std::net::SocketAddr;

use self::{
    ext::{RemoteAddrExt, RouteParamsExt},
    params::RouteParams,
};

pub trait RequestExt {
    fn params(&self) -> &RouteParams;
    fn remote_address(&self) -> &SocketAddr;
}

impl<B> RequestExt for hyper::Request<B> {
    fn params(&self) -> &RouteParams {
        &*self
            .extensions()
            .get::<RouteParamsExt>()
            .expect("missing request parameters (request not processed by routerman?)")
    }

    #[track_caller]
    fn remote_address(&self) -> &SocketAddr {
        &*self
            .extensions()
            .get::<RemoteAddrExt>()
            .expect("missing remote address (request not processed by routerman?)")
    }
}
