## MPC Node Implementation

This codebase implements a p2p mpc network for generating keypairs and sign signatures on Secp256k1 for blockchain applications. 

The core cryptographic crates are audited software with minor modification. 

## Core Crates 
|License|Crate Name|Description|Status|WASM Ready?|
|---|---|---|---|---|
|GPLv3|`crates/skw-mpc-auth`|authentication header implementing the Google Authenticator standard. |Beta|Yes|
|GPLv3|`crates/skw-mpc-client`|client side p2p node used decentralizedly relay requests to nodes.|Internal Alpha|Partially|
|GPLv3|`crates/skw-mpc-node`|p2p node runs on async runtime that handles MPC requests |Internal Alpha|No|
|GPLv3|`crates/skw-mpc-protocol`|cryptographic implementation of the mpc protocol. Partially ported from ZenGoX|Beta|N/A|
|Apache 2.0 OR GPLv3|`crates/skw-mpc-payload`|payload wrapper for messages on wire. |Beta|Yes|
|GPLv3|`crates/skw-mpc-storage`|Async levelDB wrapper|Beta|No|
|GPLv3|`crates/skw-mpc-wasm`|wasm-bindgen wrapper for mpc client|Early Internal Alpha|Yes|
|GPLv3|`crates/skw-round-based`|Async runtime for protocols with multiple rounds of communication. |Ported From ZenGoX|N/A|


## Crypto Crates 

|License|Crate Name|Description|Changes|Status|WASM Ready?|
|---|---|---|---|---|---|
|GPLv3|`crypto/skw-crypto-bulletproofs`|See crate README for details. Implements RangeProof with Bulletproof.|Only keep the core RangeProof impl|Ported from ZenGoX|Yes|
|GPLv3|`crypto/skw-crypto-centipede`|See crate README for details. Simple key generation schema.|Almost unchanged.|Ported from ZenGoX|Yes|
|MIT|`crypto/skw-crypto-curv`|See crate README for details. ecc base lib.|Allow conditional compilation for different curves|Ported from ZenGoX|Yes|
|MIT OR Apache2.0|`crypto/skw-crypto-paillier`|See crate README for details. implements the paillier cryptosystem.|Almost unchagned|Ported from ZenGoX|Yes|
|GPLv3|`crypto/skw-crypto-zk-paillier`|See crate README for details. Some zero-knolwedge proof in the paillier cryptosystem.|removed everything besides CorrectKey proof and DLogProof.|Ported from ZenGoX|Yes|

## Test Deployment Sequencing

**Step 1:** Bootstrap 2 Fullnodes
`cargo run -p skw-mpc-node --example node-full-node --release --features="full-node tcp-ws-transport" --no-default-features`. will boostrap two nodes that listen to: `/ip4/10.0.0.3/tcp/2620/ws` and `/ip4/10.0.0.3/tcp/2621/ws` with predefined keys `[0u8; 32]` and `[1u8; 32]`. You can change the configuration from `crates/skw-mpc-node/examples/node_full.rs`

**Step 2 Standalone**: You can bootstrap a client node directly without any client request relay node by `cargo run -p skw-mpc-node --example node-light-node --release --features="light-node tcp-ws-transport" --no-default-features` which will bootstrap a `light-node` that communicates with the two full node spawned from Step 1. Later, the light node starts a `KeyGen` request. Upon receiving outcome from the `KeyGen` response, it will submit `Signing` request to sign a sample hash `[2u8; 32]` and respond the resulting signature. 

**Step 2 With WebSocket Relay**: You can bootstrap a WebSocket server that wrap a `MPC Node light-node` within. Please note that this can be confusing as the `light-node` of the `mpc node` is wrapped within the `full-node` of the client node. The subsequent `light-node-client` is a request-only interface that won't be listen on any ports. 

**Step 3 With WebSocket Relay**: Run `cargo run -p skw-mpc-client --example node-full-client --release --features="full-node tcp-ws-transport" --no-default-features` to initiate a full client node that relay request to a Mpc light node. 

**Step 4 With WebSocket Relay**, With Wasm Runtime: inside the `crates/skw-mpc-wasm` and config accordingly to send or receive request in a browser. 

**Step 4 With WebSocket Relay**, Without Wasm Runtime: Run `cargo run -p skw-mpc-client --example node-light-client --release --features="light-node tcp-ws-transport" --no-default-features` to run a request client node. It will send a `KeyGen` request and a `Signing` request to message hash `[2u8; 32]`.


## For M1/M2 Mac Users 

Ref to this on [StackExchange](https://substrate.stackexchange.com/questions/1098/how-to-use-sp-core-in-libraries-that-target-wasm-for-the-web?rq=1). 


Attach `PATH="/opt/homebrew/opt/llvm/bin:$PATH" CC=/opt/homebrew/opt/llvm/bin/clang AR=/opt/homebrew/opt/llvm/bin/llvm-ar` before your cargo command.

## License

Refer to the crate list for details. Most of the codebase is licensed under GPLv3.0 with some exceptions to ported crates.


Please [contact us](https://skye.kiwi) if you have questions about
the licensing of our products.
