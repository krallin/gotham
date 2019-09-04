use futures::{Future, Stream};
use hyper::server::conn::Http;
use std::sync::Arc;
use tokio::executor;
use tokio::net::{TcpListener, TcpStream};
use tokio_io::{AsyncRead, AsyncWrite};

use super::{handler::NewHandler, service::GothamService};

#[cfg(feature = "rustls")]
pub mod test;

#[cfg(feature = "rustls")]
mod rustls_support {
    use super::super::{new_runtime, tcp_listener};
    use super::*;
    use log::info;
    use std::net::ToSocketAddrs;
    use tokio::runtime::TaskExecutor;
    use tokio_rustls::{rustls, TlsAcceptor};

    /// Starts a Gotham application with the default number of threads.
    pub fn start<NH, A>(addr: A, new_handler: NH, tls_config: rustls::ServerConfig)
    where
        NH: NewHandler + 'static,
        A: ToSocketAddrs + 'static,
    {
        start_with_num_threads(addr, new_handler, tls_config, num_cpus::get())
    }

    /// Starts a Gotham application with a designated number of threads.
    pub fn start_with_num_threads<NH, A>(
        addr: A,
        new_handler: NH,
        tls_config: rustls::ServerConfig,
        threads: usize,
    ) where
        NH: NewHandler + 'static,
        A: ToSocketAddrs + 'static,
    {
        let runtime = new_runtime(threads);
        start_on_executor(addr, new_handler, tls_config, runtime.executor());
        runtime.shutdown_on_idle().wait().unwrap();
    }

    /// Starts a Gotham application with a designated backing `TaskExecutor`.
    ///
    /// This function can be used to spawn the server on an existing `Runtime`.
    pub fn start_on_executor<NH, A>(
        addr: A,
        new_handler: NH,
        tls_config: rustls::ServerConfig,
        executor: TaskExecutor,
    ) where
        NH: NewHandler + 'static,
        A: ToSocketAddrs + 'static,
    {
        executor.spawn(init_server(addr, new_handler, tls_config));
    }

    /// Returns a `Future` used to spawn an Gotham application.
    ///
    /// This is used internally, but exposed in case the developer intends on doing any
    /// manual wiring that isn't supported by the Gotham API. It's unlikely that this will
    /// be required in most use cases; it's mainly exposed for shutdown handling.
    pub fn init_server<NH, A>(
        addr: A,
        new_handler: NH,
        tls_config: rustls::ServerConfig,
    ) -> impl Future<Item = (), Error = ()>
    where
        NH: NewHandler + 'static,
        A: ToSocketAddrs + 'static,
    {
        let listener = tcp_listener(addr);
        let addr = listener.local_addr().unwrap();

        info!(
        target: "gotham::start",
        " Gotham listening on http://{}",
        addr
        );

        bind_server_rustls(listener, new_handler, tls_config)
    }

    pub(crate) fn bind_server_rustls<NH>(
        listener: TcpListener,
        new_handler: NH,
        tls_config: rustls::ServerConfig,
    ) -> impl Future<Item = (), Error = ()>
    where
        NH: NewHandler + 'static,
    {
        let tls = TlsAcceptor::from(Arc::new(tls_config));
        bind_server(listener, new_handler, move |socket| tls.accept(socket))
    }
}

#[cfg(feature = "rustls")]
pub use rustls_support::*;

/// Returns a `Future` used to spawn a Gotham application.
///
/// This is used internally, but it's exposed for clients that want to set up their own TLS
/// support.
pub fn bind_server<NH, F, S, E, W>(
    listener: TcpListener,
    new_handler: NH,
    mut wrap: W,
) -> impl Future<Item = (), Error = ()>
where
    NH: NewHandler + 'static,
    F: Future<Item = S, Error = E> + Send + 'static,
    E: std::fmt::Debug,
    S: AsyncRead + AsyncWrite + Send + 'static,
    W: FnMut(TcpStream) -> F,
{
    let protocol = Arc::new(Http::new());
    let gotham_service = GothamService::new(new_handler);

    listener
        .incoming()
        .map_err(|e| panic!("socket error = {:?}", e))
        .for_each(move |socket| {
            let addr = socket.peer_addr().unwrap();
            let service = gotham_service.connect(addr);
            let accepted_protocol = protocol.clone();
            let handler = wrap(socket)
                .map_err(|e| panic!("https error = {:?}", e))
                .and_then(move |socket| {
                    accepted_protocol
                        .serve_connection(socket, service)
                        .map_err(|e| panic!("http error = {:?}", e))
                });

            executor::spawn(handler);

            Ok(())
        })
}
