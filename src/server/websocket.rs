use std::{sync::{mpsc, Arc, Mutex}, collections::HashMap};

use simple_websockets::{self, Message, Responder};

use crate::{cmd::ActivityCmd, logger};


#[derive(Clone)]
pub struct WebsocketConnector {
  pub client_id: String,
  pub encoding: String,
  pub server: Arc<Mutex<simple_websockets::EventHub>>,
  pub clients: Arc<Mutex<HashMap<String, Responder>>>,

  event_sender: Arc<Mutex<mpsc::Sender<ActivityCmd>>>,
}

impl WebsocketConnector {
  pub fn new(event_sender: mpsc::Sender<ActivityCmd>) -> Self {
    Self {
      client_id: "".to_string(),
      encoding: "json".to_string(),
      server: Arc::new(Mutex::new(Self::try_connect())),
      clients: Arc::new(Mutex::new(HashMap::new())),
      event_sender: Arc::new(Mutex::new(event_sender)),
    }
  }

  pub fn try_connect() -> simple_websockets::EventHub {
    for port in 6463..6473 {
      let server = simple_websockets::launch(port);
      if server.is_ok() {
        return server.unwrap();
      }
    }

    panic!("Failed to create websocket server");
  }

  pub fn start(&self) {
    // Start he event hub and have the events send to the event_sender
    let event_sender = self.event_sender.clone();
    let server = self.server.clone();
    let clients = self.clients.clone();

    std::thread::spawn(move || {
      // Send the event to the event_sender
      loop {
        match server.lock().unwrap().poll_event() {
          simple_websockets::Event::Connect(client_id, responder) => {
            logger::log(format!("Client {} connected", client_id));

            clients.lock().unwrap().insert(client_id.to_string(), responder);
          }
          simple_websockets::Event::Message(client_id, message) => {
            let msg = format!("{:?}", message);
            logger::log(format!("Client {} sent message: {:?}", client_id, msg));

            // Send the message to the event_sender
            event_sender.lock().unwrap().send(serde_json::from_str::<ActivityCmd>(&msg).unwrap()).unwrap_or_default();
          }
          simple_websockets::Event::Disconnect(client_id) => {
            logger::log(format!("Client {} disconnected", client_id));

            clients.lock().unwrap().remove(&client_id.to_string());
          }
          _ => {}
        }
      }
    });
  }

  pub fn send_data(&self, data: String) {
    let clients = self.clients.lock().unwrap();

    for (_, responder) in clients.iter() {
      responder.send(Message::Text(data.clone()));
    }
  }
}
