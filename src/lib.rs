use std::path::PathBuf;
use serde_json::Value;

pub struct RPCServer {
  socket_id: u32,
  detectable: Value,
}

impl RPCServer {
  pub fn from_str(detectable: &str) -> Self {
    // Parse as JSON, panic if invalid
    let detectable: Value = serde_json::from_str(detectable).expect("Invalid JSON provided to RPCServer");

    Self {
      socket_id: 0,
      detectable,
    }
  }

  /**
   * Create a new RPCServer and read the detectable games list from file.
   */
  pub fn from_file(file: PathBuf) -> Self {
    // Read the detectable games list from file.
    let detectable = std::fs::read_to_string(&file).expect(format!("RPCServer could not find file: {:?}", file.display()).as_str());
    let detectable: Value = serde_json::from_str(&detectable).expect("Invalid JSON provided to RPCServer");

    Self {
      socket_id: 0,
      detectable,
    }
  }

  pub fn start() {

  }
}