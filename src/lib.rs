use detection::DetectableActivity;
use serde_json::Value;
use server::{client_connector::ClientConnector, process::ProcessServer};
use std::{path::PathBuf, sync::mpsc};

mod detection;
mod logger;
mod server;

pub struct RPCServer {
  detectable: Vec<DetectableActivity>,
  process_server: ProcessServer,
  client_connector: ClientConnector,

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
      process_server: ProcessServer::new(vec![], mpsc::channel().0),
      client_connector: ClientConnector::new(1337, "".to_string()),
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
    let (proc_event_sender, proc_event_receiver) = mpsc::channel();
    self.process_server = ProcessServer::new(self.detectable, proc_event_sender);
    self.client_connector = ClientConnector::new(
      1337,
      r#"
      {
        "cmd": "DISPATCH",
        "evt": "READY",
        "data": {
          "v": 1,
          "user": {
            "id": "1045800378228281345",
            "username": "arRPC",
            "discriminator": "0000",
            "avatar": "cfefa4d9839fb4bdf030f91c2a13e95c",
            "flags": 0,
            "premium_type": 0
          },
          "config": {
            "api_endpoint": "//discord.com/api",
            "cdn_host": "cdn.discordapp.com",
            "environment": "production"
          }
        }
      }
      "#
      .to_string(),
    );

    logger::log("Starting client connector...");
    self.client_connector.start();

    if self.process_scan_ms.is_some() {
      self.process_server.scan_wait_ms = self.process_scan_ms.unwrap();
    }

    logger::log("Starting process server...");
    self.process_server.start();

    let mut last_activity: Option<DetectableActivity> = None;

    logger::log("Done! Watching for activity...");

    loop {
      let event = proc_event_receiver.recv().unwrap();
      let activity = event.activity;

      // If there are no clients, we don't care
      if self.client_connector.clients.lock().unwrap().len() == 0 {
        logger::log("No clients connected, skipping");
        continue;
      }

      match last_activity {
        Some(ref last) => {
          if activity.id == "null" {
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
          if activity.id == "null" {
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
        activity.id,
        activity.name,
        activity.timestamp.as_ref().unwrap(),
        activity.pid.unwrap_or_default(),
        activity.id
      );

      logger::log(format!("Sending payload for activity: {}", activity.name));

      last_activity = Some(activity.clone());

      // Send the empty activity to clear, then send the new activity
      self.client_connector.send_data(empty_activity(
        activity.pid.unwrap_or_default(),
        activity.id,
      ));

      self.client_connector.send_data(payload);
    }
  }
}
