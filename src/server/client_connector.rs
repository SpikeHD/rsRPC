use std::{
  collections::HashMap,
  sync::{Arc, Mutex},
};

use simple_websockets::{Event, EventHub, Message, Responder};

use crate::logger;

#[derive(Clone)]
pub struct ClientConnector {
  pub port: u16,
  server: Arc<Mutex<EventHub>>,
  pub clients: Arc<Mutex<HashMap<u64, Responder>>>,
  data_on_connect: String,
}

impl ClientConnector {
  pub fn new(port: u16, data_on_connect: String) -> ClientConnector {
    ClientConnector {
      server: Arc::new(Mutex::new(simple_websockets::launch(port).unwrap())),
      clients: Arc::new(Mutex::new(HashMap::new())),
      data_on_connect,
      port,
    }
  }

  pub fn start(&self) {
    let clone = self.clone();
    let clients_clone = self.clients.clone();

    std::thread::spawn(move || {
      loop {
        match clone.server.lock().unwrap().poll_event() {
          Event::Connect(client_id, responder) => {
            logger::log(format!("Client {} connected", client_id));

            // Send initial connection data
            responder.send(Message::Text(clone.data_on_connect.clone()));

            clients_clone.lock().unwrap().insert(client_id, responder);
          }
          Event::Disconnect(client_id) => {
            clients_clone.lock().unwrap().remove(&client_id);
          }
          Event::Message(client_id, message) => {
            logger::log(format!(
              "Received message from client {}: {:?}",
              client_id, message
            ));
            let responder = clients_clone.lock().unwrap();
            let responder = responder.get(&client_id).unwrap();
            responder.send(message);
          }
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
    drop(self.server.lock().unwrap());
  }
}