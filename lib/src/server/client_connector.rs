use std::{
  collections::HashMap,
  sync::{Arc, Mutex},
};

use serde::Serialize;
use serde_with::skip_serializing_none;
use simple_websockets::{Event, EventHub, Message, Responder};

use crate::{
  cmd::{ActivityCmd, ActivityPayload},
  detection::NULL_ACTIVITY_ID,
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
      server: Arc::new(Mutex::new(match simple_websockets::launch(port) {
        Ok(server) => Some(server),
        Err(_) => {
          log!("[Client Connector] Failed to launch websocket server on port {}, port may already be in use", port);
          None
        }
      })),
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
    let server = match self.server.lock().unwrap().take() {
      Some(server) => server,
      None => {
        log!("[Client Connector] Nothing to start");
        return;
      }
    };
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

            match clients.get(&client_id) {
              None => log!(
                "[Client Connector] Message from unregistered client {}, skipping",
                client_id
              ),
              Some(responder) => {
                responder.send(message);
              }
            }
          }
        }
      }
    });

    // Each source connector gets its own thread, feeding clients with activity
    self.listen_for_activity("IPC".to_string(), self.ipc_event_rec.clone());
    self.listen_for_activity("Websocket".to_string(), self.ws_event_rec.clone());
    self.listen_for_process_events();
  }

  /**
   * Take a source's receiver and spawn the thread that forwards its events.
   */
  fn listen_for_activity(
    &mut self,
    source: String,
    source_rec: Arc<Mutex<Option<std::sync::mpsc::Receiver<ActivityCmd>>>>,
  ) {
    let event_rec = match source_rec.lock().unwrap().take() {
      Some(rec) => rec,
      None => {
        log!("[Client Connector] {} receiver already started", source);
        return;
      }
    };
    let mut clone = self.clone();

    std::thread::spawn(move || {
      while let Ok(event) = event_rec.recv() {
        clone.forward_activity(event, source.clone());
      }
    });
  }

  pub fn forward_activity(&mut self, mut event: ActivityCmd, source: String) {
    // if there are no clients, skip
    if self.clients.lock().unwrap().is_empty() {
      log!("[Client Connector] No clients connected, skipping");
      return;
    }

    event.fix();

    let mut args = match event.args {
      Some(args) => args,
      None => {
        log!("[Client Connector] Invalid activity command, skipping");
        return;
      }
    };

    if args.activity.is_none() {
      let pid = args.pid.unwrap_or_default();

      log!("[Client Connector] Sending empty payload");
      self.send_data(empty_activity(pid, pid.to_string()));

      return;
    }

    let activity = args.activity.as_mut();

    if let Some(activity) = activity {
      activity.application_id = event.application_id;

      let payload = ActivityPayload {
        activity: Some(activity.clone()),
        pid: args.pid,
        socket_id: Some(args.pid.unwrap_or(0).to_string()),
      };

      match serde_json::to_string(&payload) {
        Ok(payload) => {
          log!(
            "[Client Connector] Sending payload for {} activity: {:?}",
            source,
            payload
          );
          self.send_data(payload);
        }
        Err(err) => log!(
          "[Client Connector] Error serializing {} activity: {}",
          source,
          err
        ),
      };
    } else {
      log!("[Client Connector] Invalid activity command, skipping");
    }
  }

  /**
   * Forward process-scan events, one "socket" per detected activity.
   */
  fn listen_for_process_events(&mut self) {
    let proc_event_rec = match self.proc_event_rec.lock().unwrap().take() {
      Some(rec) => rec,
      None => {
        log!("[Client Connector] Process receiver already taken, listener already started");
        return;
      }
    };
    let mut clone = self.clone();

    std::thread::spawn(move || {
      while let Ok(proc_event) = proc_event_rec.recv() {
        let proc_activity = proc_event.activity;

        // if there are no clients, skip
        if clone.clients.lock().unwrap().is_empty() {
          log!("[Client Connector] No clients connected, skipping");
          continue;
        }

        if proc_activity.id == NULL_ACTIVITY_ID {
          if let Some(socket) = clone.active_socket.clone() {
            log!("[Client Connector] Sending empty payload");
            clone.send_data(empty_activity(clone.last_pid.unwrap_or_default(), socket));
            clone.active_socket = None;
          }

          continue;
        }

        // Same activity as last time, nothing new to report
        let previous_socket = clone.active_socket.clone();

        if previous_socket == Some(proc_activity.id.clone()) {
          log!(
            "[Client Connector] Already sent payload for activity: {}",
            proc_activity.name
          );
          continue;
        }

        // A different activity takes over, so the old socket gets cleared first
        if let Some(socket) = previous_socket {
          log!("[Client Connector] Sending empty payload");
          clone.send_data(empty_activity(clone.last_pid.unwrap_or_default(), socket));
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

        clone.last_pid = proc_activity.pid;
        clone.active_socket = Some(proc_activity.id.clone());

        log!(
          "[Client Connector] Sending payload for activity: {}",
          proc_activity.name
        );

        match serde_json::to_string(&payload_struct) {
          Ok(payload) => clone.send_data(payload),
          Err(err) => log!(
            "[Client Connector] Error serializing process activity: {}",
            err
          ),
        }
      }
    });
  }

  pub fn send_data(&mut self, data: String) {
    // Send data to all clients
    for responder in self.clients.lock().unwrap().values() {
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
