# ironbar-ipc-types

Types for [Ironbar](https://crates.io/crates/ironbar)'s IPC protocol, shared across other crates.

These types can be used by other crates to communicate with Ironbar.

A base `Ipc` struct is provided which holds the socket path.
Currently, send/receive methods must be implemented manually. 
Reference implementations can be found in [`ironbar-cli`](https://crates.io/crates/ironbar-cli) and [`ironbar`](https://crates.io/crates/ironbar) respectively.

Requests & responses are sent in a JSON format, 
and can be serialized/deserialized using the `serde_json` crate.