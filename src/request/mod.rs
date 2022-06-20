pub(crate) mod ext;
pub(crate) mod params;

use std::net::SocketAddr;

use crate::Request;

use self::{
    ext::{RemoteAddrExt, RouteParamsExt},
    params::RouteParams,
};

pub trait RequestExt {
    fn params(&self) -> &RouteParams;

    fn remote_address(&self) -> &SocketAddr;
}

impl RequestExt for Request {
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
