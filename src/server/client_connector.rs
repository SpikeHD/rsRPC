use std::net::SocketAddr;
use std::error::Error;
use std::time::Duration;
use futures_util::{SinkExt, StreamExt};
use async_tungstenite::accept_async;
use port_scanner::local_port_available;
use async_std::net::{TcpListener, TcpStream};
use async_std::task::{self, block_on};

use super::base::BaseServer;

pub struct WebsocketServer {
  base: BaseServer,
  port: u16,
}

impl WebsocketServer {
  pub fn new() -> Result<Self, std::io::Error> {
    let port = 1337;

    Ok(Self {
      base: BaseServer::new(),
      port,
    })
  }

  pub fn start(self) -> Result<(), Box<dyn Error>> {
    let listener = block_on(TcpListener::bind(format!("127.0.0.1:{}", self.port))).unwrap();

    println!("Websocket server started on port: {}", self.port);

    while let Ok((stream, addr)) = block_on(listener.accept()) {
      task::spawn(Self::handle_connection(stream, addr));
    }

    Ok(())
  }

  async fn handle_connection(stream: TcpStream, addr: SocketAddr) {
    println!("Websocket connection established!");
    
    let ws_stream = accept_async(stream).await.expect("Failed to accept");

    // Keep stream open
    let (mut write, mut read) = ws_stream.split();
    
    while let Some(msg) = read.next().await {
      let msg = msg.expect("Failed to get message");
      println!("Received a message from {}: {}", addr, msg);
      //write.send(msg).await.expect("Failed to send message");
    }
  }
}