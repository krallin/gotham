//! A Hello World example application for working with Gotham.
use failure::{err_msg, Error};
use std::sync::Arc;
use openssl::{
    pkey::PKey,
    ssl::{SslAcceptor, SslMethod},
    x509::X509,
};
use std::net::ToSocketAddrs;
use tokio::{net::TcpListener, runtime::Runtime};

use gotham::{bind_server, state::State};

const HELLO_WORLD: &str = "Hello World!";

pub fn say_hello(state: State) -> (State, &'static str) {
    (state, HELLO_WORLD)
}

/// Create an OpenSSL acceptor, then set up Gotham to use it.
#[tokio::main]
pub async fn main() -> Result<(), Error> {
    let addr = "127.0.0.1:7878";
    println!("Listening for requests at https://{}", addr);
    let acceptor = Arc::new(build_acceptor()?);

    let addr = addr
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| err_msg("Invalid Socket Address"))?;

    let listener = TcpListener::bind(&addr).await?;

    let server = bind_server(
        listener,
        || Ok(say_hello),
        move |socket| {
            let acceptor = acceptor.clone();
            async move {
                // NOTE: We're ignoring handshake errors here. You can modify to e.g. report them.
                tokio_openssl::accept(&acceptor, socket).await.map_err(|_| ())
            }
        },
    );

    let mut runtime = Runtime::new()?;
    runtime
        .block_on(server)
        .map_err(|()| err_msg("Server failed"))
}

fn build_acceptor() -> Result<SslAcceptor, Error> {
    let cert = X509::from_pem(&include_bytes!("cert.pem")[..])?;
    let pkey = PKey::private_key_from_pem(&include_bytes!("key.pem")[..])?;

    let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls())?;
    builder.set_certificate(&cert)?;
    builder.set_private_key(&pkey)?;
    Ok(builder.build())
}
