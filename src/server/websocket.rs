use std::{
  collections::HashMap,
  sync::{mpsc, Arc, Mutex},
};

use serde::{Deserialize, Serialize};
use simple_websockets::{Event, EventHub, Message, Responder};

use crate::log;

#[derive(Clone, Serialize, Deserialize)]
pub struct WebsocketEvent {
  pub cmd: String,
  pub args: HashMap<String, String>,
  pub data: HashMap<String, String>,
  pub evt: String,
  pub nonce: String,
}

pub struct WebsocketConnector {
  pub port: u16,
  server: Arc<Mutex<EventHub>>,
  pub clients: Arc<Mutex<HashMap<u64, Responder>>>,

  event_sender: mpsc::Sender<WebsocketEvent>,
}

impl WebsocketConnector {
  pub fn new(port: u16, event_sender: mpsc::Sender<WebsocketEvent>) -> WebsocketConnector {
    WebsocketConnector {
      server: Arc::new(Mutex::new(simple_websockets::launch(port).unwrap_or_else(|_| {
        log!("[Websocket] Failed to launch websocket server, port may already be in use");
        std::process::exit(1);
      }))),
      clients: Arc::new(Mutex::new(HashMap::new())),
      port,
      event_sender,
    }
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
            log!("[Websocket] Client {} connected", client_id);
            
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