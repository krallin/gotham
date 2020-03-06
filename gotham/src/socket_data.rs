//! Defines a `SocketData` type for passing state from the incoming socket to the `State`.

use crate::state::State;

/// SocketData represents data extracted from a Socket before HTTP requests are dispatched through
/// Hyper.
pub trait SocketData {
    /// Inject this SocketData into the State. This takes a reference to Self, because multiple
    /// requests might be executed on a single socket.
    fn populate_state(&self, state: &mut State);
}

impl SocketData for () {
    fn populate_state(&self, _state: &mut State) {
        // No-op
    }
}
