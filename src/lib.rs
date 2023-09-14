use std::{path::PathBuf, sync::mpsc};
use detection::DetectableActivity;
use serde_json::Value;
use server::{process::ProcessServer, client_connector::ClientConnector};

mod server;
mod detection;


pub struct RPCServer {
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
      detectable,
      process_scan_ms: None,
    }
  }

  pub fn start(self) {
    let (proc_event_sender, proc_event_receiver) = mpsc::channel();
    let mut process_server = ProcessServer::new(self.detectable, proc_event_sender);
    let mut client_connector = ClientConnector::new(
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
      "#.to_string(),
    );

    client_connector.start();

    if self.process_scan_ms.is_some() {
      process_server.scan_wait_ms = self.process_scan_ms.unwrap();
    }

    process_server.start();

    let mut last_activity: Option<DetectableActivity> = None;

    loop {
      let event = proc_event_receiver.recv().unwrap();
      let activity = event.activity;
      
      match last_activity {
        Some(ref last) => {
          if activity.id == "null" {
              // Send empty payload
            let payload = format!(r#"
              {{
                "activity": null,
                "pid": {},
                "socketId": "{}"
              }}
            "#, last.pid.unwrap_or_default(), last.id);

            println!("Sending empty payload");

            client_connector.send_data(payload);

            continue;
          }
        },
        None => {
          // We haven't had any activities yet :(
          if activity.id == "null" {
            continue;
          } 
        },
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
        "#, activity.id, activity.name, activity.timestamp.as_ref().unwrap(), activity.pid.unwrap_or_default(), activity.id
      );

      last_activity = Some(activity.clone());

      client_connector.send_data(payload);
    };
  }
}