# P2P_CHAT

A work in progress slack like p2p webchat using webrtc for client to client communication and websocket to enter the network.

Hyper and tungstenite for the server, binary protocol for server/client and client/client communication and wasm for the fron end.

## Requirements
 - [Rust toolchain](https://rustup.rs/)
 - [Wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)
## Install

```bash
// build
$> make build

// run
$> make run
```

http://localhost:8088