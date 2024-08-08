use std::{
  collections::HashMap,
  sync::{Arc, Mutex},
};

use simple_websockets::{Event, EventHub, Message, Responder};

use crate::log;

pub struct WebsocketConnector {
  pub port: u16,
  server: Arc<Mutex<EventHub>>,
  pub clients: Arc<Mutex<HashMap<u64, Responder>>>,
}

impl WebsocketConnector {
  pub fn new(port: u16) -> WebsocketConnector {
    WebsocketConnector {
      server: Arc::new(Mutex::new(simple_websockets::launch(port).unwrap())),
      clients: Arc::new(Mutex::new(HashMap::new())),
      port,
    }
  }

  pub fn start(&self) {
    // let server = self.server.clone();
    // let clients = self.clients.clone();

    // std::thread::spawn(move || {
    //   let mut server = server.lock().unwrap();
    //   let mut clients = clients.lock().unwrap();

    //   loop {

    //     match server.poll_event() {
    //       Event::Connect(client_id, responder) => {
    //         log!("Client {} connected", client_id);
            
    //         clients.insert(client_id, responder);
    //       }
    //       Event::Disconnect(client_id) => {
    //         clients.remove(&client_id);
    //       }
    //       Event::Message(client_id, message) => {
    //         log!(
    //           "Received message from client {}: {:?}",
    //           client_id, message
    //         );
    //         let responder = clients;
    //         let responder = responder.get(&client_id).unwrap();
    //         responder.send(message);
    //       }
    //     }

    //     std::thread::sleep(std::time::Duration::from_millis(100));
    //   }
    // });
  }
}