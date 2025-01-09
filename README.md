<div align=center>
  <h1>rsRPC</h1>

  <div align="center">
    <img src="https://img.shields.io/github/actions/workflow/status/SpikeHD/rsRPC/build.yml" />
    <img src="https://img.shields.io/github/actions/workflow/status/SpikeHD/rsRPC/code_quality.yml?label=code quality" />
    <img src="https://img.shields.io/github/repo-size/SpikeHD/rsRPC" />
  </div>
  <p>Alternative Discord RPC server CLI tool and Rust library, inspired by <a href="https://github.com/OpenAsar/arRPC">arRPC</a></p>
</div>

# Features

* Process detection
* IPC/Socket-based RPC detection
* Websocket-based RPC detection
* `INVITE_BROWSER` support
* Adding new processes on the fly
* Manually triggering scans

# Building

## Requirements

- [Cargo and Rust](https://www.rust-lang.org/)

## Testing it out

1. Download a binary from [releases](https://github.com/SpikeHD/rsRPC/releases), [GitHub Actions](https://www.github.com/SpikeHD/rsRPC/actions) or build it yourself below!
2. If you just want to use the default detectable list, just run the binary!
3. If you want to use your own detectable list, place a `detectable.json` file in the same directory as the binary (you can use [the arRPC one](https://raw.githubusercontent.com/OpenAsar/arrpc/main/src/process/detectable.json) as an example), then run the binary with `./rsrpc-cli -d ./detectable.json`

## Building the binary

1. Clone the repository
2. `cargo build -p rsrpc-cli --release`
3. Your file will be in `target/release/`

## Using as a library

1. Add the following to your `Cargo.toml` file:

```toml
[dependencies]
rsrpc = { git = "https://www.github.com/SpikeHD/rsRPC", tag = "VERSION_NUMBER_HERE" }
```

2. Use the library in your code:

```rust
use rsrpc::{RPCServer, RPCConfig};

fn main() {
  let mut server = RPCServer::from_file("./detectable.json", RPCConfig::default());
  server.start();
}
```

You can also grab the `detectable.json` programmatically and pass it via string:
```rust
use rsrpc::{RPCServer, RPCConfig};

fn main() {
  let detectable = reqwest::blocking::get("https://raw.githubusercontent.com/OpenAsar/arrpc/main/src/process/detectable.json")?.text()?;
  let mut server = RPCServer::from_json_str(detectable, RPCConfig::default());

  server.start();
}
```
