Rust Chat Server
================

This is a small chat server in [Rust](https://www.rust-lang.org/). It manages all connected clients. Messages received from a client are distributed to all other clients and printed to the server console.

As there is no dedicated client software (yet?), you should connect to it with netcat or telnet, e.g. `nc 127.0.0.1 1337`.

Import the repository as an eclipse project, using the [RustDT plugin](https://github.com/RustDT/RustDT).