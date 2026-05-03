use std::{
  collections::HashMap,
  sync::{Arc, Mutex},
};

use serde::Serialize;
use serde_with::skip_serializing_none;
use simple_websockets::{Event, EventHub, Message, Responder};

use crate::{
  cmd::{ActivityCmd, ActivityPayload},
  log,
};

use super::process::ProcessDetectedEvent;

fn empty_activity(pid: u64, socket_id: String) -> String {
  format!(
    r#"
    {{
      "activity": null,
      "pid": {pid},
      "socketId": "{socket_id}"
    }}
  "#
  )
}

#[derive(Clone)]
pub struct ClientConnector {
  pub port: u16,
  server: Arc<Mutex<Option<EventHub>>>,
  pub clients: Arc<Mutex<HashMap<u64, Responder>>>,
  data_on_connect: String,

  pub last_pid: Option<u64>,
  pub active_socket: Option<String>,

  pub ipc_event_rec: Arc<Mutex<Option<std::sync::mpsc::Receiver<ActivityCmd>>>>,
  pub proc_event_rec: Arc<Mutex<Option<std::sync::mpsc::Receiver<ProcessDetectedEvent>>>>,
  pub ws_event_rec: Arc<Mutex<Option<std::sync::mpsc::Receiver<ActivityCmd>>>>,
}

#[skip_serializing_none]
#[derive(Serialize)]
struct ProcessActivity {
  pub application_id: String,
  pub name: String,
  pub timestamps: ProcessTimestamps,
  pub r#type: u32,
  pub metadata: HashMap<String, String>,
  pub flags: u32,
}

#[derive(Serialize)]
struct ProcessTimestamps {
  pub start: String,
}

#[derive(Serialize)]
struct ProcessPayload {
  pub activity: ProcessActivity,
  pub pid: u64,
  #[serde(rename = "socketId")]
  pub socket_id: String,
}

impl ClientConnector {
  pub fn new(
    port: u16,
    data_on_connect: String,
    ipc_event_rec: std::sync::mpsc::Receiver<ActivityCmd>,
    proc_event_rec: std::sync::mpsc::Receiver<ProcessDetectedEvent>,
    ws_event_rec: std::sync::mpsc::Receiver<ActivityCmd>,
  ) -> ClientConnector {
    ClientConnector {
      server: Arc::new(Mutex::new(Some(
        simple_websockets::launch(port).unwrap_or_else(|_| {
          log!("[Client Connector] Failed to launch websocket server, port may already be in use");
          std::process::exit(1);
        }),
      ))),
      clients: Arc::new(Mutex::new(HashMap::new())),
      data_on_connect,
      port,

      last_pid: None,
      active_socket: None,

      ipc_event_rec: Arc::new(Mutex::new(Some(ipc_event_rec))),
      proc_event_rec: Arc::new(Mutex::new(Some(proc_event_rec))),
      ws_event_rec: Arc::new(Mutex::new(Some(ws_event_rec))),
    }
  }

  pub fn start(&mut self) {
    let server = self
      .server
      .lock()
      .unwrap()
      .take()
      .expect("Client connector already started");
    let clients_clone = self.clients.clone();
    let data_on_connect = self.data_on_connect.clone();

    std::thread::spawn(move || {
      loop {
        match server.poll_event() {
          Event::Connect(client_id, responder) => {
            log!("[Client Connector] Client {} connected", client_id);

            // Send initial connection data
            responder.send(Message::Text(data_on_connect.clone()));

            clients_clone.lock().unwrap().insert(client_id, responder);
          }
          Event::Disconnect(client_id) => {
            clients_clone.lock().unwrap().remove(&client_id);
          }
          Event::Message(client_id, message) => {
            log!(
              "[Client Connector] Received message from client {}: {:?}",
              client_id,
              message
            );
            let clients = clients_clone.lock().unwrap();
            if let Some(responder) = clients.get(&client_id) {
              responder.send(message);
            }
          }
        }
      }
    });

    // Create a thread for each reciever
    let ipc_event_rec = self.ipc_event_rec.lock().unwrap().take().unwrap();
    let proc_event_rec = self.proc_event_rec.lock().unwrap().take().unwrap();
    let ws_event_rec = self.ws_event_rec.lock().unwrap().take().unwrap();

    let mut ipc_clone = self.clone();
    let mut proc_clone = self.clone();
    let mut ws_clone = self.clone();

    std::thread::spawn(move || {
      while let Ok(mut ipc_activity) = ipc_event_rec.recv() {
        // if there are no client, skip
        if ipc_clone.clients.lock().unwrap().is_empty() {
          log!("[Client Connector] No clients connected, skipping");
          continue;
        }

        ipc_activity.fix();

        let mut args = match ipc_activity.args {
          Some(args) => args,
          None => {
            log!("[Client Connector] Invalid activity command, skipping");
            continue;
          }
        };

        if args.activity.is_none() {
          let pid = args.pid.unwrap_or_default();
          // Send empty payload
          let payload = empty_activity(pid, pid.to_string());

          log!("[Client Connector] Sending empty payload");

          ipc_clone.send_data(payload);

          continue;
        }

        let activity = args.activity.as_mut();

        if let Some(activity) = activity {
          activity.application_id = ipc_activity.application_id;

          let payload = ActivityPayload {
            activity: Some(activity.clone()),
            pid: args.pid,
            socket_id: Some(args.pid.unwrap_or(0).to_string()),
          };

          match serde_json::to_string(&payload) {
            Ok(payload) => {
              log!(
                "[Client Connector] Sending payload for IPC activity: {:?}",
                payload
              );
              ipc_clone.send_data(payload)
            }
            Err(err) => log!("[Client Connector] Error serializing IPC activity: {}", err),
          };
        } else {
          log!("[Client Connector] Invalid activity command, skipping");
        }
      }
    });

    std::thread::spawn(move || {
      while let Ok(proc_event) = proc_event_rec.recv() {
        let proc_activity = proc_event.activity;

        // if there are no clients, skip
        if proc_clone.clients.lock().unwrap().is_empty() {
          log!("[Client Connector] No clients connected, skipping");
          continue;
        }

        if proc_activity.id == "null" {
          // If our last socket id is empty, skip
          if proc_clone.active_socket.is_none() {
            continue;
          }

          // Send an empty payload
          log!("[Client Connector] Sending empty payload");

          let payload = empty_activity(
            proc_clone.last_pid.unwrap_or_default(),
            proc_clone.active_socket.as_ref().unwrap().clone(),
          );

          proc_clone.send_data(payload);

          proc_clone.active_socket = None;

          continue;
        }

        // If the active socket is different from the current socket, send an empty payload for the old socket
        if proc_clone.active_socket != Some(proc_activity.id.clone()) {
          if proc_clone.active_socket.is_some() {
            // Send an empty payload
            log!("[Client Connector] Sending empty payload");

            let payload = empty_activity(
              proc_clone.last_pid.unwrap_or_default(),
              proc_clone.active_socket.as_ref().unwrap().clone(),
            );

            proc_clone.send_data(payload);
          }
        } else {
          log!(
            "[Client Connector] Already sent payload for activity: {}",
            proc_activity.name
          );
          continue;
        }

        let payload_struct = ProcessPayload {
          activity: ProcessActivity {
            application_id: proc_activity.id.clone(),
            name: proc_activity.name.clone(),
            timestamps: ProcessTimestamps {
              start: proc_activity
                .timestamp
                .as_ref()
                .cloned()
                .unwrap_or_else(|| "0".to_string()),
            },
            r#type: 0,
            metadata: HashMap::new(),
            flags: 0,
          },
          pid: proc_activity.pid.unwrap_or_default(),
          socket_id: proc_activity.id.clone(),
        };

        proc_clone.last_pid = proc_activity.pid;
        proc_clone.active_socket = Some(proc_activity.id.clone());

        log!(
          "[Client Connector] Sending payload for activity: {}",
          proc_activity.name
        );

        match serde_json::to_string(&payload_struct) {
          Ok(payload) => proc_clone.send_data(payload),
          Err(err) => log!(
            "[Client Connector] Error serializing process activity: {}",
            err
          ),
        }
      }
    });

    std::thread::spawn(move || {
      while let Ok(mut ws_event) = ws_event_rec.recv() {
        // if there are no clients, skip
        if ws_clone.clients.lock().unwrap().is_empty() {
          log!("[Client Connector] No clients connected, skipping");
          continue;
        }

        if ws_event.cmd != "SET_ACTIVITY" {
          // Just send the event as-is, there isn't really anything to go off of here
          // I will change this if arRPC implements things like INVITE_BROWSER event responses, to ensure compatibility
          let payload = serde_json::to_string(&ws_event).unwrap_or("".to_string());
          log!("[Client Connector] Sending payload for WS event");
          ws_clone.send_data(payload);

          continue;
        }

        ws_event.fix();

        let mut args = match ws_event.args {
          Some(args) => args,
          None => {
            log!("[Client Connector] Invalid activity command, skipping");
            continue;
          }
        };

        if args.activity.is_none() {
          let pid = args.pid.unwrap_or_default();
          // Send empty payload
          let payload = empty_activity(pid, pid.to_string());

          log!("[Client Connector] Sending empty payload");

          ws_clone.send_data(payload);

          continue;
        }

        let activity = args.activity.as_mut();

        if let Some(activity) = activity {
          activity.application_id = ws_event.application_id;

          let payload = ActivityPayload {
            activity: Some(activity.clone()),
            pid: args.pid,
            socket_id: Some(args.pid.unwrap_or(0).to_string()),
          };

          match serde_json::to_string(&payload) {
            Ok(payload) => {
              log!(
                "[Client Connector] Sending payload for WS activity: {:?}",
                payload
              );
              ws_clone.send_data(payload)
            }
            Err(err) => log!("[Client Connector] Error serializing WS activity: {}", err),
          };
        } else {
          log!("[Client Connector] Invalid activity command, skipping");
        }
      }
    });
  }

  pub fn send_data(&mut self, data: String) {
    // Send data to all clients
    for (_, responder) in self.clients.lock().unwrap().iter() {
      responder.send(Message::Text(data.clone()));
    }
  }
}

impl Drop for ClientConnector {
  fn drop(&mut self) {
    if let Ok(mut server) = self.server.lock() {
      drop(server.take());
    }
  }
}
