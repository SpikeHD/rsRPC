use port_scanner::local_port_available;

use super::base::BaseServer;

pub struct WebsocketServer {
  base: BaseServer,
  port: u16,
}

impl WebsocketServer {
  pub fn new() -> Self {
    let port_range = (6463, 6472);
    let mut port = port_range.0;

    while !local_port_available(port) {
      port += 1;
      if port > port_range.1 {
        panic!("No available port in range: {:?}", port_range);
      }
    }

    Self {
      base: BaseServer::new(),
      port,
    }
  }
}