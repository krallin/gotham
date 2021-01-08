//! Defines the `GothamService` type which is used to wrap a Gotham application and interface with
//! Hyper.

use std::net::SocketAddr;
use std::panic::AssertUnwindSafe;
use std::pin::Pin;
use std::sync::Arc;
use std::thread;

use futures::prelude::*;
use futures::task::{self, Poll};
use http::request;
use hyper::service::Service;
use hyper::{Body, Request, Response};
use log::debug;

use crate::handler::NewHandler;

use crate::helpers::http::request::path::RequestPathSegments;
use crate::socket_data::SocketData;
use crate::state::client_addr::put_client_addr;
use crate::state::{set_request_id, State};

mod trap;

/// A `Handler` which has been connected to a client.
pub(crate) struct ConnectedGothamService<T, S>
where
    T: NewHandler + 'static,
{
    handler: Arc<T>,
    client_addr: SocketAddr,
    socket_data: S,
}

impl<T, S> ConnectedGothamService<T, S>
where
    T: NewHandler + 'static,
{
    pub fn connect(handler: Arc<T>, client_addr: SocketAddr, socket_data: S) -> Self {
        ConnectedGothamService {
            handler,
            client_addr,
            socket_data,
        }
    }
}

impl<T, S> Service<Request<Body>> for ConnectedGothamService<T, S>
where
    T: NewHandler,
    S: SocketData,
{
    type Response = Response<Body>;
    type Error = anyhow::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(
        &mut self,
        _cx: &mut task::Context<'_>,
    ) -> Poll<std::result::Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call<'a>(&'a mut self, req: Request<Body>) -> Self::Future {
        let mut state = State::new();

        put_client_addr(&mut state, self.client_addr);
        self.socket_data.populate_state(&mut state);

        let (
            request::Parts {
                method,
                uri,
                version,
                headers,
                //extensions?
                ..
            },
            body,
        ) = req.into_parts();

        state.put(RequestPathSegments::new(uri.path()));
        state.put(method);
        state.put(uri);
        state.put(version);
        state.put(headers);
        state.put(body);

        {
            let request_id = set_request_id(&mut state);
            debug!(
                "[DEBUG][{}][Thread][{:?}]",
                request_id,
                thread::current().id(),
            );
        };

        trap::call_handler(self.handler.clone(), AssertUnwindSafe(state)).boxed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use hyper::{Body, StatusCode};

    use crate::helpers::http::response::create_empty_response;
    use crate::router::builder::*;
    use crate::state::State;

    fn handler(state: State) -> (State, Response<Body>) {
        let res = create_empty_response(&state, StatusCode::ACCEPTED);
        (state, res)
    }

    #[test]
    fn new_handler_closure() {
        let req = Request::get("http://localhost/")
            .body(Body::empty())
            .unwrap();
        let f = ConnectedGothamService::connect(
            Arc::new(|| Ok(handler)),
            "127.0.0.1:10000".parse().unwrap(),
            (),
        )
        .call(req);
        let response = futures::executor::block_on(f).unwrap();
        assert_eq!(response.status(), StatusCode::ACCEPTED);
    }

    #[test]
    fn router() {
        let router = build_simple_router(|route| {
            route.get("/").to(handler);
        });

        let req = Request::get("http://localhost/")
            .body(Body::empty())
            .unwrap();
        let f = ConnectedGothamService::connect(
            Arc::new(router),
            "127.0.0.1:10000".parse().unwrap(),
            (),
        )
        .call(req);
        let response = futures::executor::block_on(f).unwrap();
        assert_eq!(response.status(), StatusCode::ACCEPTED);
    }
}
