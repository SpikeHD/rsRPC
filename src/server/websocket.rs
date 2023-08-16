use std::net::SocketAddr;
use std::error::Error;
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
    let port_range = ( 6463, 6472 );
    let mut port = 0;

    // Scan for available ports in range
    for p in port_range.0..port_range.1 {
      if local_port_available(port) {
        port = p;
        println!("Found available port: {}", port);
        break;
      }
    }

    if port == 0 {
      panic!("No port available in range: {:?}", port_range);
    }

    port = 1337;

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

    // Write some data to the stream
    let (mut write, _) = ws_stream.split();
  
    let s = "{
      \"cmd\": \"DISPATCH\",
      \"evt\": \"READY\",
      \"data\": {
        \"v\": 1,
        \"user\": {
          \"id\": \"1045800378228281345\",
          \"username\": \"arRPC\",
          \"discriminator\": \"0000\",
          \"avatar\": \"cfefa4d9839fb4bdf030f91c2a13e95c\",
          \"flags\": 0,
          \"premium_type\": 0
        },
        \"config\": {
          \"api_endpoint\": \"//discord.com/api\",
          \"cdn_host\": \"cdn.discordapp.com\",
          \"environment\": \"production\"
        }
      }
    }";
    
    write.send(s.into()).await.expect("Failed to send");
  }
}