pub enum PacketType {
  Handshake,
  Frame,
  Close,
  Ping,
  Pong,
}

impl PacketType {
  pub fn from_u32(value: u32) -> Self {
    match value {
      0 => PacketType::Handshake,
      1 => PacketType::Frame,
      2 => PacketType::Close,
      3 => PacketType::Ping,
      4 => PacketType::Pong,
      _ => panic!("Invalid packet type: {}", value),
    }
  }
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Handshake {
  pub v: u32,
  pub client_id: String,
}
