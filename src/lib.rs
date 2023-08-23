use std::path::PathBuf;
use detection::DetectableActivity;
use serde_json::Value;
use server::{process::ProcessServer, client_connector::{ClientConnector, self}};

mod server;
mod detection;


pub struct RPCServer {
  socket_id: u32,
  detectable: Vec<DetectableActivity>,

  // Milliseconds to wait between each processes scan. Good for limiting CPU usage.
  pub process_scan_ms: Option<u64>,
}

impl RPCServer {
  pub fn from_str(detectable: &str) -> Self {
    // Parse as JSON, panic if invalid
    let detectable: Value = serde_json::from_str(detectable).expect("Invalid JSON provided to RPCServer");

    // Turn detectable into a vector of DetectableActivity
    let detectable: Vec<DetectableActivity> = detectable.as_array().unwrap().iter().map(|x| {
      serde_json::from_value(x.clone()).unwrap()
    }).collect();

    Self {
      socket_id: 0,
      detectable,
      process_scan_ms: None,
    }
  }

  /**
   * Create a new RPCServer and read the detectable games list from file.
   */
  pub fn from_file(file: PathBuf) -> Self {
    // Read the detectable games list from file.
    let detectable = std::fs::read_to_string(&file).expect(format!("RPCServer could not find file: {:?}", file.display()).as_str());
    let detectable: Value = serde_json::from_str(&detectable).expect("Invalid JSON provided to RPCServer");

    // Turn detectable into a vector of DetectableActivity
    let detectable: Vec<DetectableActivity> = detectable.as_array().unwrap().iter().map(|x| {serde_json::from_value(x.clone()).unwrap()}).collect();

    Self {
      socket_id: 0,
      detectable,
      process_scan_ms: None,
    }
  }

  pub fn start(self) {
    let mut process_server = ProcessServer::new(self.detectable);
    let mut client_connector = ClientConnector::new(1337);

    client_connector.start();

    if self.process_scan_ms.is_some() {
      process_server.scan_wait_ms = self.process_scan_ms.unwrap();
    }

    let process_thread = std::thread::spawn(move || {
      process_server.start();
    });
  }
}