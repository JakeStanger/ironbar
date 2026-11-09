# ironbar-cli

Ironbar CLI implementation. 

This is the entrypoint binary for [Ironbar](https://crates.io/crates/ironbar), responsible for:

- Sending requests to Ironbar's IPC.
- Forwarding the `exec` to the main `ironbard` binary (implemented in the primary crate).