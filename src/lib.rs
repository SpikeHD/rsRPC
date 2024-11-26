use detection::DetectableActivity;
use serde_json::Value;
use server::{
  client_connector::ClientConnector,
  ipc::IpcConnector,
  process::{ProcessEventListeners, ProcessScanState, ProcessServer},
  websocket::WebsocketConnector,
};
use std::{
  path::PathBuf,
  sync::{mpsc, Arc, Mutex},
};

pub mod cmd;
pub mod detection;
mod logger;
mod server;
mod url_params;

pub type ProcessCallback = dyn FnMut(ProcessScanState) + Send + Sync;

#[derive(Clone, Debug)]
pub struct RPCConfig {
  pub enable_process_scanner: bool,
  pub enable_ipc_connector: bool,
  pub enable_websocket_connector: bool,
  pub enable_secondary_events: bool,
}

impl Default for RPCConfig {
  fn default() -> Self {
    Self {
      enable_process_scanner: true,
      enable_ipc_connector: true,
      enable_websocket_connector: true,
      enable_secondary_events: true,
    }
  }
}

#[derive(Clone)]
pub struct Connectors {
  process_server: Arc<Mutex<ProcessServer>>,
  client_connector: Arc<Mutex<ClientConnector>>,
  ipc_connector: Arc<Mutex<IpcConnector>>,
  ws_connector: Arc<Mutex<WebsocketConnector>>,
}

pub struct RPCServer {
  detectable: Arc<Mutex<Vec<DetectableActivity>>>,
  connectors: Option<Connectors>,
  config: RPCConfig,

  on_process_scan_complete: Option<Arc<Mutex<ProcessCallback>>>,
}

impl RPCServer {
  pub fn from_json_str(
    detectable: impl AsRef<str>,
    config: RPCConfig,
  ) -> Result<Self, Box<dyn std::error::Error>> {
    // Parse as JSON, panic if invalid
    let detectable: Value =
      serde_json::from_str(detectable.as_ref()).expect("Invalid JSON provided to RPCServer");

    // Turn detectable into a vector of DetectableActivity
    let detectable_arr = detectable.as_array();
    let detectable: Vec<DetectableActivity>;

    if let Some(detectable_arr) = detectable_arr {
      detectable = detectable_arr
        .iter()
        .map(|x| serde_json::from_value(x.clone()).expect("Detectable list malformed!"))
        .collect();
    } else {
      log!("Detectable list empty!");
      detectable = vec![];
    }

    Ok(Self {
      detectable: Arc::new(Mutex::new(detectable)),

      // Default to empty servers
      connectors: None,
      config,

      // Event listeners
      on_process_scan_complete: None,
    })
  }

  /**
   * Create a new RPCServer and read the detectable games list from file.
   */
  pub fn from_file(file: PathBuf, config: RPCConfig) -> Result<Self, Box<dyn std::error::Error>> {
    // Read the detectable games list from file.
    let detectable = std::fs::read_to_string(&file)
      .unwrap_or_else(|_| panic!("RPCServer could not find file: {:?}", file.display()));

    Self::from_json_str(detectable.as_str(), config)
  }

  /**
   * Add new detectable processes on-the-fly. This should be run AFTER start().
   */
  pub fn append_detectables(&mut self, detectable: Vec<DetectableActivity>) {
    if self.connectors.is_none() {
      log!("[RPC Server] Cannot append detectables, connectors are not initialized");
      return;
    }

    self
      .connectors
      .as_mut()
      .unwrap()
      .process_server
      .lock()
      .unwrap()
      .append_detectables(detectable);
  }

  /**
   * Remove a detectable process by name.
   */
  pub fn remove_detectable_by_name(&mut self, name: String) {
    if self.connectors.is_none() {
      log!("[RPC Server] Cannot remove detectable, connectors are not initialized");
      return;
    }

    self
      .connectors
      .as_mut()
      .unwrap()
      .process_server
      .lock()
      .unwrap()
      .remove_detectable_by_name(name);
  }

  /**
   * Manually trigger a scan for processes. This should be run AFTER start().
   */
  pub fn scan_for_processes(&mut self) {
    if self.connectors.is_none() {
      log!("[RPC Server] Cannot scan processes, connectors are not initialized");
      return;
    }

    let process_server = self
      .connectors
      .as_mut()
      .unwrap()
      .process_server
      .lock()
      .unwrap();

    match process_server.scan_for_processes() {
      Ok(_) => {}
      Err(err) => {
        log!("[RPC Server] Error while scanning processes: {}", err);
      }
    }
  }

  pub fn on_process_scan_complete(
    &mut self,
    callback: impl FnMut(ProcessScanState) + Send + Sync + 'static,
  ) {
    if self.connectors.is_some() {
      log!("[RPC Server] Cannot set on_streamer_mode_should_toggle, connectors are already initialized");
      return;
    }

    self.on_process_scan_complete = Some(Arc::new(Mutex::new(callback)));
  }

  pub fn start(&mut self) {
    let (proc_event_sender, proc_event_receiver) = mpsc::channel();
    let (ipc_event_sender, ipc_event_receiver) = mpsc::channel();
    let (ws_event_sender, ws_event_reciever) = mpsc::channel();

    let connectors = Connectors {
      process_server: Arc::new(Mutex::new(ProcessServer::new(
        self.detectable.lock().unwrap().to_vec(),
        proc_event_sender,
        8,
        ProcessEventListeners {
          on_process_scan_complete: self.on_process_scan_complete.clone(),
        },
      ))),
      client_connector: Arc::new(Mutex::new(ClientConnector::new(
        1337,
        server::utils::CONNECTION_REPONSE.to_string(),
        ipc_event_receiver,
        proc_event_receiver,
        ws_event_reciever,
      ))),
      ipc_connector: Arc::new(Mutex::new(IpcConnector::new(ipc_event_sender))),
      ws_connector: Arc::new(Mutex::new(WebsocketConnector::new(ws_event_sender))),
    };

    log!(
      "[RPC Server] Starting client connector on port {}...",
      connectors.client_connector.lock().unwrap().port
    );
    connectors.client_connector.lock().unwrap().start();

    let config = self.config.clone();

    if config.enable_ipc_connector {
      log!("[RPC Server] Starting IPC connector...");
      connectors.ipc_connector.lock().unwrap().start();
    }

    if config.enable_process_scanner {
      log!("[RPC Server] Starting process server...");
      connectors.process_server.lock().unwrap().start();
    }

    if config.enable_websocket_connector || config.enable_secondary_events {
      log!("[RPC Server] Starting websocket connector...");
      connectors.ws_connector.lock().unwrap().start(
        config.enable_websocket_connector,
        config.enable_secondary_events,
      );
    }

    log!("[RPC Server] Done! Watching for activity...");
    self.connectors = Some(connectors);
  }
}
