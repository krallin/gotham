//! An introduction to sharing state across handlers in a safe way.
//!
//! This example demonstrates a basic request counter which can be
//! used across server threads, and be used to track the number of
//! requests sent to the backend.

#![cfg_attr(feature = "cargo-clippy", allow(clippy::mutex_atomic))]
extern crate gotham;
#[macro_use]
extern crate gotham_derive;
extern crate failure;
extern crate futures;
extern crate hyper;
extern crate mime;
extern crate tokio;

use failure::{err_msg, Error};
use futures::IntoFuture;
use gotham::bind_server_with_pre_state;
use gotham::state::{FromState, State};
use gotham::PreStateData;
use std::net::SocketAddr;
use std::net::ToSocketAddrs;
use tokio::{net::TcpListener, runtime::Runtime};

struct LocalAddrPreStateData {
    local_addr: SocketAddr,
}

impl PreStateData for LocalAddrPreStateData {
    fn fill_state(&self, state: &mut State) {
        let val = LocalAddrStateData {
            local_addr: self.local_addr,
        };
        state.put(val);
    }
}

#[derive(Clone, StateData)]
struct LocalAddrStateData {
    local_addr: SocketAddr,
}

fn say_hello(state: State) -> (State, String) {
    let addr = LocalAddrStateData::borrow_from(&state).local_addr;
    let message = format!("You are connected to {:?}\n", addr);
    (state, message)
}

/// Start a server and call the `Handler` we've defined above
/// for each `Request` we receive.
pub fn main() -> Result<(), Error> {
    let addr = "127.0.0.1:7878";
    println!("Listening for requests at http://{}", addr);

    let addr = addr
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| err_msg("Invalid Socket Address"))?;

    let listener = TcpListener::bind(&addr)?;

    let server = bind_server_with_pre_state(
        listener,
        || Ok(say_hello),
        move |socket| {
            // NOTE: Typically, you'd want to return Err here instead.
            let local_addr = socket.local_addr().unwrap();
            let pre_state_data = LocalAddrPreStateData { local_addr };
            Ok((socket, pre_state_data)).into_future()
        },
    );

    let mut runtime = Runtime::new()?;

    runtime
        .block_on(server)
        .map_err(|()| err_msg("Server failed"))
}
