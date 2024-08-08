use std::{
  collections::HashMap,
  sync::{mpsc, Arc, Mutex},
};

use serde::{Deserialize, Serialize};
use simple_websockets::{ConnectionDetails, Event, EventHub, Message, Responder};

use crate::{log, server::utils::CONNECTION_REPONSE, url_params::get_url_params};

#[derive(Clone, Serialize, Deserialize)]
pub struct WebsocketEvent {
  pub cmd: String,
  pub args: Option<HashMap<String, String>>,
  pub data: Option<HashMap<String, String>>,
  pub evt: Option<String>,
  pub nonce: String,
}

pub struct WebsocketConnector {
  pub port: u16,
  server: Arc<Mutex<EventHub>>,
  pub clients: Arc<Mutex<HashMap<u64, Responder>>>,

  event_sender: mpsc::Sender<WebsocketEvent>,
}

impl WebsocketConnector {
  pub fn new(event_sender: mpsc::Sender<WebsocketEvent>) -> WebsocketConnector {
    // Try starting websocket server on ports 6463 - 6472
    for port in 6463..6472 {
      match simple_websockets::launch(port) {
        Ok(server) => {
          log!("[Websocket] Server started on port {}", port);
          return WebsocketConnector {
            server: Arc::new(Mutex::new(server)),
            clients: Arc::new(Mutex::new(HashMap::new())),
            port,
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

    std::thread::spawn(move || {
      let server = server.lock().unwrap();
      let mut clients = clients.lock().unwrap();

      loop {
        match server.poll_event() {
          Event::Connect(client_id, responder) => {
            let connection = responder.connection_details();
            let url_params = get_url_params(connection.uri.clone());
            let version = url_params.get("v").unwrap_or(&"0".to_string()).clone();
            let encoding = url_params.get("encoding").unwrap_or(&"json".to_string()).clone();

            log!("[Websocket] Client {} connected", client_id);
        
            if version != "1" || encoding != "json" {
              log!("[Websocket] Invalid connection from client {}", client_id);
              return;
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
              client_id, message
            );
            let responder = clients.get(&client_id).unwrap();
            responder.send(message);
          }
        }

        std::thread::sleep(std::time::Duration::from_millis(100));
      }
    });
  }
}