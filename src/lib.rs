use detection::DetectableActivity;
use serde_json::Value;
use server::{client_connector::ClientConnector, process::ProcessServer, ipc::IpcConnector};
use std::{path::PathBuf, sync::mpsc};

pub mod detection;
pub mod cmd;
mod logger;
mod server;

pub struct RPCServer {
  detectable: Vec<DetectableActivity>,
  process_server: ProcessServer,
  client_connector: ClientConnector,
  ipc_connector: IpcConnector,

  // Milliseconds to wait between each processes scan. Good for limiting CPU usage.
  pub process_scan_ms: Option<u64>,
}

fn empty_activity(pid: u64, socket_id: String) -> String {
  format!(
    r#"
    {{
      "activity": null,
      "pid": {},
      "socketId": "{}"
    }}
  "#,
    pid, socket_id
  )
}

impl RPCServer {
  pub fn from_json_str(detectable: impl AsRef<str>) -> Self {
    // Parse as JSON, panic if invalid
    let detectable: Value =
      serde_json::from_str(detectable.as_ref()).expect("Invalid JSON provided to RPCServer");

    // Turn detectable into a vector of DetectableActivity
    let detectable: Vec<DetectableActivity> = detectable
      .as_array()
      .unwrap()
      .iter()
      .map(|x| serde_json::from_value(x.clone()).unwrap())
      .collect();

    Self {
      detectable,
      process_scan_ms: None,

      // These are default empty servers, and are replaced on start()
      process_server: ProcessServer::new(vec![], mpsc::channel().0, 1),
      client_connector: ClientConnector::new(65447, "".to_string()),
      ipc_connector: IpcConnector::new(mpsc::channel().0),
    }
  }

  /**
   * Create a new RPCServer and read the detectable games list from file.
   */
  pub fn from_file(file: PathBuf) -> Self {
    // Read the detectable games list from file.
    let detectable = std::fs::read_to_string(&file)
      .unwrap_or_else(|_| panic!("RPCServer could not find file: {:?}", file.display()));

    Self::from_json_str(detectable.as_str())
  }

  /**
   * Add new detectable processes on-the-fly. This should be run AFTER start().
   */
  pub fn append_detectables(&mut self, detectable: Vec<DetectableActivity>) {
    self.process_server.append_detectables(detectable);
  }

  pub fn start(mut self) {
    // Ensure the IPC socket is closed
    self.ipc_connector.close();

    let (proc_event_sender, proc_event_receiver) = mpsc::channel();
    let (ipc_event_sender, ipc_event_receiver) = mpsc::channel();

    self.process_server = ProcessServer::new(self.detectable, proc_event_sender, 8);
    self.ipc_connector = IpcConnector::new(ipc_event_sender);
    self.client_connector = ClientConnector::new(
      1337,
      server::utils::connection_resp()
      .to_string(),
    );

    logger::log(format!("Starting client connector on port {}...", self.client_connector.port));
    self.client_connector.start();

    logger::log("Starting IPC connector...");
    self.ipc_connector.start();

    if self.process_scan_ms.is_some() {
      self.process_server.scan_wait_ms = self.process_scan_ms.unwrap();
    }

    logger::log("Starting process server...");
    self.process_server.start();

    let mut last_activity: Option<DetectableActivity> = None;

    logger::log("Done! Watching for activity...");

    loop {
      let proc_event = proc_event_receiver.recv().unwrap();
      let proc_activity = proc_event.activity;

      // If there are no clients, we don't care
      if self.client_connector.clients.lock().unwrap().len() == 0 {
        logger::log("No clients connected, skipping");
        continue;
      }

      match last_activity {
        Some(ref last) => {
          if proc_activity.id == "null" {
            // Send empty payload
            let payload = format!(
              r#"
              {{
                "activity": null,
                "pid": {},
                "socketId": "{}"
              }}
            "#,
              last.pid.unwrap_or_default(),
              last.id
            );

            logger::log("Sending empty payload");

            self.client_connector.send_data(payload);

            continue;
          }
        }
        None => {
          // We haven't had any activities yet :(
          if proc_activity.id == "null" {
            continue;
          }
        }
      }

      let payload = format!(
        // I don't even know what half of these fields are for yet
        r#"
        {{
          "activity": {{
            "application_id": "{}",
            "name": "{}",
            "timestamps": {{
              "start": {}
            }},
            "type": 0,
            "metadata": {{}},
            "flags": 0
          }},
          "pid": {},
          "socketId": "{}"
        }}
        "#,
        proc_activity.id,
        proc_activity.name,
        proc_activity.timestamp.as_ref().unwrap(),
        proc_activity.pid.unwrap_or_default(),
        proc_activity.id
      );

      logger::log(format!("Sending payload for activity: {}", proc_activity.name));

      // Send the empty activity to clear, then send the new activity
      if let Some(ref last) = last_activity {
        let empty_payload = empty_activity(last.pid.unwrap_or_default(), last.id.clone());

        self.client_connector.send_data(empty_payload);
      }

      last_activity = Some(proc_activity.clone());

      self.client_connector.send_data(payload);
    }
  }
}
