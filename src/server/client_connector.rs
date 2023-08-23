use std::net::TcpListener;

use websocket::{server::{sync::Server, WsServer, NoTlsAcceptor}, Message};

pub struct ClientConnector {
  port: u16,
  server: Option<WsServer<NoTlsAcceptor, TcpListener>>,
  writer: Option<websocket::sender::Writer<std::net::TcpStream>>,
  connection_data: Option<String>,
}

impl ClientConnector {
  pub fn new(port: u16) -> ClientConnector {
    ClientConnector {
      port,
      server: None,
      writer: None,
      connection_data: None,
    }
  }

  pub fn start(&mut self) {
    self.server = Some(Server::bind(format!("127.0.0.1:{}", self.port)).unwrap());
    self.server.as_mut().unwrap().set_nonblocking(true).unwrap();

    let connection_data = self.connection_data.as_ref().unwrap().clone();

    // Whenever we get a connection, assign the writer to the writer field.
    for request in self.server.as_mut().unwrap().filter_map(Result::ok) {
      let result = match request.accept() {
        Ok(wsupgrade) => {
          let (mut _read, write) = wsupgrade.split().unwrap();

          self.writer = Some(write);
          Ok(())
        },
        Err(_) => {
          println!("Error accepting connection");
          Err(())
        },
      };
    }
  }

  pub fn send_data(&mut self, data: String) {
    if self.writer.is_some() {
      self.writer.as_mut().unwrap().send_message(&Message::text(data)).unwrap();
    }
  }
}