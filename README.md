<div align=center>
  <h1>rsRPC</h1>

  <p>Discord RPC server binary and library, lovingly written in Rust using the core research from <a href="https://github.com/OpenAsar/arRPC">arRPC</a> <sup>(love you guys 💕)</sup></p>
</div>

# Building

## Requirements

- [Cargo and Rust](https://www.rust-lang.org/)

## Testing it out

1. Download a binary from [actions](https://www.github.com/SpikeHD/rsRPC/actions), from the [releases](https://www.github.com/SpikeHD/rsRPC/releases) page, or build it yourself below!
2. Place a `detectable.json` file in the same directory as the binary (you can use [the arRPC one](https://raw.githubusercontent.com/OpenAsar/arrpc/main/src/process/detectable.json) as an example)
3. Run the binary with `./rsrpc -d ./detectable.json`

## Building the binary

1. Clone the repository
2. Place a `detectable.json` file in the root folder (you can use [the arRPC one](https://raw.githubusercontent.com/OpenAsar/arrpc/main/src/process/detectable.json) as an example)
3. If you wanna try it out, run `cargo run --features binary -- -d ./detectable.json` in the root directory
4. If you want to make a build, run `cargo build --features binary` in the root directory
5. The binary will be in `target/release/rsrpc`

## Using as a library

1. Add the following to your `Cargo.toml` file:

```toml
[dependencies]
rsrpc = { git = "https://www.github.com/SpikeHD/rsRPC" }
```

2. Use the library in your code:

```rust
use rsrpc::RPCServer;

fn main() {
  let mut server = RPCServer::from_file("./detectable.json").unwrap();

  // This is optional, but highly reccommended. It will change the buffer time in between each process in the process scan, which is trigger once every 5 seconds.
  server.process_scan_ms = 100;

  server.start().unwrap();
}
```

You can also grab the `detectable.json` programmatically and pass it via string:
```rust
fn main() {
  let detectable = reqwest::blocking::get("https://raw.githubusercontent.com/OpenAsar/arrpc/main/src/process/detectable.json").unwrap().text().unwrap();

  // This accepts both a `&str` or a `String`
  let mut server = RPCServer::from_json_str(detectable).unwrap();

  server.start().unwrap();
}
```