use std::{
  collections::HashMap,
  sync::{mpsc, Arc, Mutex},
};

use serde::{Deserialize, Serialize};
use simple_websockets::{Event, EventHub, Message, Responder};

use crate::{log, server::utils::CONNECTION_REPONSE, url_params::get_url_params};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebsocketEvent {
  pub cmd: String,
  pub args: Option<HashMap<String, String>>,
  pub data: Option<HashMap<String, String>>,
  pub evt: Option<String>,
  pub nonce: String,
}

#[derive(Clone)]
pub struct WebsocketConnector {
  server: Arc<Mutex<EventHub>>,
  pub clients: Arc<Mutex<HashMap<u64, Responder>>>,

  event_sender: mpsc::Sender<WebsocketEvent>,
}

impl WebsocketConnector {
  pub fn new(event_sender: mpsc::Sender<WebsocketEvent>) -> Self {
    // Try starting websocket server on ports 6463 - 6472
    for port in 6463..6472 {
      match simple_websockets::launch(port) {
        Ok(server) => {
          log!("[Websocket] Server started on port {}", port);
          return Self {
            server: Arc::new(Mutex::new(server)),
            clients: Arc::new(Mutex::new(HashMap::new())),
            event_sender,
          };
        }
        Err(_) => {
          log!("[Websocket] Failed to start server on port {}", port);
        }
      }
    }

    log!("[Websocket] Failed to start server on any port");
    std::process::exit(1);
  }

  pub fn start(&self) {
    let server = self.server.clone();
    let clients = self.clients.clone();
    let event_sender = self.event_sender.clone();

    std::thread::spawn(move || {
      let server = server.lock().unwrap();
      let mut clients = clients.lock().unwrap();

      loop {
        log!("[Websocket] Polling for events...");

        match server.poll_event() {
          Event::Connect(client_id, responder) => {
            let connection = responder.connection_details();
            let url_params = get_url_params(connection.uri.clone());
            let version = url_params.get("v").unwrap_or(&"0".to_string()).clone();
            let encoding = url_params
              .get("encoding")
              .unwrap_or(&"json".to_string())
              .clone();

            log!("[Websocket] Client {} connected", client_id);

            if version != "1" || encoding != "json" {
              log!("[Websocket] Invalid connection from client {}", client_id);
              continue;
            }

            responder.send(Message::Text(CONNECTION_REPONSE.to_string()));

            clients.insert(client_id, responder);
          }
          Event::Disconnect(client_id) => {
            log!("[Websocket] Client {} disconnected", client_id);
            clients.remove(&client_id);
          }
          Event::Message(client_id, message) => {
            log!(
              "[Websocket] Received message from client {}: {:?}",
              client_id,
              message
            );

            let responder = clients.get(&client_id).unwrap();
            let message = match message {
              Message::Text(text) => text,
              _ => "".to_string(),
            };

            // If not WebsocketEvent, ignore
            let event: WebsocketEvent = match serde_json::from_str(&message) {
              Ok(event) => event,
              Err(_) => {
                log!("[Websocket] Invalid message from client {}", client_id);
                continue;
              }
            };

            // If origin isn't a Discord URL, ignore
            let origin = responder.connection_details().headers.get("origin");

            if let Some(origin) = origin {
              let value = origin.to_str().unwrap_or_default();
              let valid = [
                "https://discord.com",
                "https://canary.discord.com",
                "https://ptb.discord.com",
              ];

              if !valid.contains(&value) {
                log!("[Websocket] Invalid origin from client {}", client_id);
                continue;
              }
            }

            match event.cmd.as_str() {
              "INVITE_BROWSER" => handle_invite(&event, &event_sender, responder),
              "DEEP_LINK" => {
                log!("[Websocket] Deep link unimplemented. PRs are open!");
              }
              _ => (),
            }
          }
        }
      }
    });
  }
}

fn handle_invite(
  event: &WebsocketEvent,
  event_sender: &mpsc::Sender<WebsocketEvent>,
  responder: &Responder,
) {
  // Let's just assume this went well I don't care
  let response = WebsocketEvent {
    cmd: event.cmd.clone(),
    args: None,
    data: event.args.clone(),
    evt: None,
    nonce: event.nonce.clone(),
  };

  // Send the event away!
  event_sender.send(event.clone()).unwrap();

  // Respond
  responder.send(Message::Text(serde_json::to_string(&response).unwrap()));
}
