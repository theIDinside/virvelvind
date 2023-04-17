## Completing the Maelstrom mini-challenges using Rust

This repo is to complete the fun challenges found at [fly.io's maelstrom challenges](https://fly.io/dist-sys/)


### Formats of the Message types

Example of the `EchoServiceDefinition` which are called `Definition` due to this being the type
that represents what kind of protocol or message payloads it's dealing in. If it's not defined
here, the node is going to crash when attempting to serialize input; i.e. it doesn't protect itself
from faulty input; Maelstrom is not going to send it any.

```rust
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")] // make enum "internally tagged"
pub enum EchoServiceDefinition {
    Echo { echo: String },
    EchoOk { echo: String },
}
```

The enums need to have serde rename them when deserialized to `type` using `$[serde(tag="type")]`.
For the challanges, the individual requests and responses that _are_ defined by maelstrom, we either
have to name the variants identically to what maelstrom expects (like `init_ok` instead of `InitOk`) or
provide the serde rename functionality as in this example.

For our own custom messages and requests (i.e. the requests that we pass between nodes) we can call them
whatever we want.

I might write a macro to simplify this, but it's so hard to write decent rust macros that it might
just not even be worth the time. If *you* are the one defining the protocol, why make it general or generic?