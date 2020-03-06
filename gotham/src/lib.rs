//! Gotham &ndash; A flexible web framework that promotes stability, safety, security and speed.
//!
//! You can find out more about Gotham, including where to get help, at <https://gotham.rs>.
//!
//! We look forward to welcoming you into the Gotham community!
#![doc(html_root_url = "https://docs.rs/gotham/0.5.0")] // Update when changed in Cargo.toml
#![warn(missing_docs, deprecated)]
// Stricter requirements once we get to pull request stage, all warnings must be resolved.
#![cfg_attr(feature = "ci", deny(warnings))]
#![cfg_attr(
    feature = "cargo-clippy",
    allow(
        clippy::needless_lifetimes,
        clippy::should_implement_trait,
        clippy::unit_arg,
        clippy::match_wild_err_arm,
        clippy::new_without_default,
        clippy::wrong_self_convention,
        clippy::mutex_atomic,
        clippy::borrowed_box,
        clippy::get_unwrap,
    )
)]
#![doc(test(no_crate_inject, attr(deny(warnings))))]
// TODO: Remove this when it's a hard error by default (error E0446).
// See Rust issue #34537 <https://github.com/rust-lang/rust/issues/34537>
#![deny(private_in_public)]

pub mod extractor;
pub mod handler;
pub mod helpers;
pub mod middleware;
pub mod pipeline;
pub mod router;
mod service;
pub mod socket_data;
pub mod state;

/// Test utilities for Gotham and Gotham consumer apps.
pub mod test;

/// Functions for creating a Gotham service using HTTP.
pub mod plain;

/// Functions for creating a Gotham service using HTTPS.
#[cfg(feature = "rustls")]
pub mod tls;

/// Re-export anyhow
pub use anyhow;
/// Re-export hyper
pub use hyper;

/// Re-export rustls
#[cfg(feature = "rustls")]
pub use tokio_rustls::rustls;

use futures::prelude::*;
use hyper::server::conn::Http;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, TcpStream};

use tokio::runtime::{self, Runtime};

use crate::socket_data::SocketData;
use crate::{handler::NewHandler, service::ConnectedGothamService};

pub use plain::*;
#[cfg(feature = "rustls")]
pub use tls::start as start_with_tls;

fn new_runtime(threads: usize) -> Runtime {
    runtime::Builder::new()
        .threaded_scheduler()
        .core_threads(threads)
        .thread_name("gotham-worker")
        .enable_all()
        .build()
        .unwrap()
}

async fn tcp_listener<A>(addr: A) -> std::io::Result<TcpListener>
where
    A: ToSocketAddrs + 'static,
{
    let addr = addr
        .to_socket_addrs()
        .expect("unable to parse listener address")
        .next()
        .expect("unable to resolve listener address");

    TcpListener::bind(addr).await
}

/// Returns a `Future` used to spawn a Gotham application.
///
/// This is used internally, but it's exposed for clients that want to set up their own TLS
/// support. The wrap argument is a function that will receive a tokio-io TcpStream and should wrap
/// the socket as necessary. Errors returned by this function will be ignored and the connection
/// will be dropped if the future returned by the wrapper resolves to an error.
pub async fn bind_server<'a, NH, Wrap, WrapFut, Wrapped>(
    listener: TcpListener,
    handler: NH,
    wrap: Wrap,
) -> Result<(), ()>
where
    NH: NewHandler + 'static,
    Wrap: Fn(TcpStream) -> WrapFut,
    WrapFut: Future<Output = Result<Wrapped, ()>> + Send + 'static,
    Wrapped: Unpin + AsyncRead + AsyncWrite + Send + 'static,
{
    bind_server_with_socket_data(listener, handler, move |sock| {
        let wrapped = wrap(sock);
        async {
            let socket = wrapped.await?;
            Result::<_, ()>::Ok(((), socket))
        }
    })
    .await
}

/// Similar to bind_server, but allows capturing `SocketData` when wrapping the socket, so that it
/// can be passed through to the State.
pub async fn bind_server_with_socket_data<'a, NH, Wrap, WrapFut, Wrapped, S>(
    mut listener: TcpListener,
    handler: NH,
    wrap: Wrap,
) -> Result<(), ()>
where
    NH: NewHandler + 'static,
    S: SocketData + Send + 'static,
    Wrap: Fn(TcpStream) -> WrapFut,
    WrapFut: Future<Output = Result<(S, Wrapped), ()>> + Send + 'static,
    Wrapped: Unpin + AsyncRead + AsyncWrite + Send + 'static,
{
    let protocol = Arc::new(Http::new());
    let handler = Arc::new(handler);

    listener
        .incoming()
        .filter_map(|sock| match sock {
            Ok(sock) => future::ready(Some(sock)),
            Err(e) => {
                panic!("socket error = {:?}", e);
            }
        })
        .for_each(|socket| {
            let protocol = protocol.clone();
            let handler = handler.clone();

            let addr = socket.peer_addr().unwrap();
            let wrapper = wrap(socket);

            // NOTE: HTTP protocol errors and handshake errors are ignored here (i.e. so the socket
            // will be dropped).
            let task = async move {
                let (socket_data, socket) = wrapper.await?;

                let service = ConnectedGothamService::connect(handler, addr, socket_data);

                protocol
                    .serve_connection(socket, service)
                    .with_upgrades()
                    .map_err(|_| ())
                    .await?;

                Result::<_, ()>::Ok(())
            };

            tokio::spawn(task);

            future::ready(())
        })
        .await;

    Ok(())
}
