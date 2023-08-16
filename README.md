<div align=center>
  <h1>rsRPC</h1>

  <p>Discord RPC server binary and library, lovingly written in Rust using the core research from <a href="https://github.com/OpenAsar/arRPC">arRPC</a></p>
</div>

# Building

## Requirements

- [Cargo and Rust](https://www.rust-lang.org/)

## Testing it out

1. Clone the repository
2. Place a `detectable.json` file in the root folder (you can use [the arRPC one](https://raw.githubusercontent.com/OpenAsar/arrpc/main/src/process/detectable.json) as an example)
3. Run `cargo run --features binary -- -d ./detectable.json` in the root directory

## Building the binary

1. Clone the repository
2. Run `cargo build --features binary` in the root directory
3. The binary will be in `target/release/rsrpc`