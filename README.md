# Nanors #

nanors is meant to be a simple software wallet that works with the [Nano network](https://nano.org/). I have been working on this project in order to learn more about cryptography, cryptocurrency, Nano and programming in Rust. 

------


Wallet operations are preformed within the [tokio runtime](https://github.com/tokio-rs/tokio).

All account balances are currently in raw.


-------

## Usage

`cargo run`

or 

`cargo build --release` and run executable in target directory. 

---------

## Features
- seed and account generation
- transacting on accounts 
- local block signing
- local proof of work 
- local wallet encryption (aes_gcm)
- rpc client for interacting with the network
- websocket client for observing the network.
- raw/MNano unit conversion 
  
## Roadmap
- account work caching with tasks.
- bip39 and seed import
- CLI, manager, wallet in separate project
- wallet file convention, use OS app dir.
- handle sigterm in CLI send, change
- CLI set manager node.

## Acknowledgements

- [feeless](https://github.com/feeless/feeless) ideas and bits of code including nano base32 encoding, raw.
- [dalek-cryptography](https://github.com/dalek-cryptography/ed25519-dalek) ed25519 key gen, signing.
- [dialoguer](https://github.com/mitsuhiko/dialoguer) command line prompts and similar things.
- [nano ninja](https://mynano.ninja/) public nano node.
- [SomeNano](https://blog.nano.org/getting-started-developing-with-nano-currency-part-2-interacting-with-public-and-private-nano-adb98ef57fbf) thoughts on developing with nano.
- [Nano.Net](https://github.com/miguel1117/Nano.Net) a reference for some code.

