use detection::DetectableActivity;
use serde_json::Value;
use server::{client_connector::ClientConnector, ipc::IpcConnector, process::ProcessServer};
use std::{
  path::PathBuf,
  sync::{mpsc, Arc, Mutex},
};

pub mod cmd;
pub mod detection;
mod logger;
mod server;

pub struct RPCServer {
  detectable: Arc<Mutex<Vec<DetectableActivity>>>,
  process_server: Arc<Mutex<ProcessServer>>,
  client_connector: Arc<Mutex<ClientConnector>>,
  ipc_connector: Arc<Mutex<IpcConnector>>,
}

impl RPCServer {
  pub fn from_json_str(detectable: impl AsRef<str>) -> Result<Self, Box<dyn std::error::Error>> {
    // Parse as JSON, panic if invalid
    let detectable: Value =
      serde_json::from_str(detectable.as_ref()).expect("Invalid JSON provided to RPCServer");

    // Turn detectable into a vector of DetectableActivity
    let detectable_arr = detectable.as_array();
    let detectable: Vec<DetectableActivity>;

    if let Some(detectable_arr) = detectable_arr {
      detectable = detectable_arr.iter()
        .map(|x| serde_json::from_value(x.clone()).expect("Detectable list malformed!"))
        .collect();
    } else {
      println!("Detectable list empty!");
      detectable = vec![];
    }

    Ok(Self {
      detectable: Arc::new(Mutex::new(detectable)),

      // These are default empty servers, and are replaced on start()
      process_server: Arc::new(Mutex::new(ProcessServer::new(vec![], mpsc::channel().0, 1))),
      client_connector: Arc::new(Mutex::new(ClientConnector::new(
        65447,
        "".to_string(),
        mpsc::channel().1,
        mpsc::channel().1,
      ))),
      ipc_connector: Arc::new(Mutex::new(IpcConnector::new(mpsc::channel().0))),
    })
  }

  /**
   * Create a new RPCServer and read the detectable games list from file.
   */
  pub fn from_file(file: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
    // Read the detectable games list from file.
    let detectable = std::fs::read_to_string(&file)
      .unwrap_or_else(|_| panic!("RPCServer could not find file: {:?}", file.display()));

    Ok(Self::from_json_str(detectable.as_str())?)
  }

  /**
   * Add new detectable processes on-the-fly. This should be run AFTER start().
   */
  pub fn append_detectables(&mut self, detectable: Vec<DetectableActivity>) {
    self
      .process_server
      .lock()
      .unwrap()
      .append_detectables(detectable);
  }

  pub fn start(mut self) {
    // Ensure the IPC socket is closed
    self.ipc_connector.lock().unwrap().close();

    let (proc_event_sender, proc_event_receiver) = mpsc::channel();
    let (ipc_event_sender, ipc_event_receiver) = mpsc::channel();

    self.process_server = Arc::new(Mutex::new(ProcessServer::new(
      self.detectable.lock().unwrap().to_vec(),
      proc_event_sender,
      8,
    )));
    self.ipc_connector = Arc::new(Mutex::new(IpcConnector::new(ipc_event_sender)));
    self.client_connector = Arc::new(Mutex::new(ClientConnector::new(
      1337,
      server::utils::connection_resp().to_string(),
      ipc_event_receiver,
      proc_event_receiver,
    )));

    logger::log(format!(
      "Starting client connector on port {}...",
      self.client_connector.lock().unwrap().port
    ));
    self.client_connector.lock().unwrap().start();

    logger::log("Starting IPC connector...");
    self.ipc_connector.lock().unwrap().start();

    logger::log("Starting process server...");
    self.process_server.lock().unwrap().start();

    logger::log("Done! Watching for activity...");
  }
}
