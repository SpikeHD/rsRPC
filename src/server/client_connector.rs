use std::{collections::HashMap, sync::{Mutex, Arc}};

use simple_websockets::{Event, Responder, EventHub, Message};

pub struct ClientConnector {
  server: Arc<Mutex<EventHub>>,
  clients: HashMap<u64, Responder>,
  data_on_connect: String,
}

impl ClientConnector {
  pub fn new(port: u16, data_on_connect: String) -> ClientConnector {
    ClientConnector {
      server: Arc::new(Mutex::new(simple_websockets::launch(port).unwrap())),
      clients: HashMap::new(),
      data_on_connect: data_on_connect,
    }
  }

  pub fn start(mut self) {
    std::thread::spawn(move || {
      loop {
        match self.server.lock().unwrap().poll_event() {
          Event::Connect(client_id, responder) => {
            println!("Client {} connected", client_id);

            // Send initial connection data
            responder.send(Message::Text(self.data_on_connect.clone()));

            self.clients.insert(client_id, responder);
          },
          Event::Disconnect(client_id) => {
            self.clients.remove(&client_id);
          },
          Event::Message(client_id, message) => {
            println!("Received message from client {}: {:?}", client_id, message);
            let responder = self.clients.get(&client_id).unwrap();
            responder.send(message);
          },
        }
      }
    });
  }

  pub fn send_data(&mut self, data: String) {
    // Send data to all clients
    for (_, responder) in self.clients.iter() {
      responder.send(Message::Text(data.clone()));
    }
  }
}