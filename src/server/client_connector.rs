use websocket::server::WsServer;

pub struct WebsocketServer {
  port: u16,
}

impl WebsocketServer {
  pub fn new(port: u16) -> WebsocketServer {
    WebsocketServer { port }
  }

  pub fn start(&self) {
     
  }
}