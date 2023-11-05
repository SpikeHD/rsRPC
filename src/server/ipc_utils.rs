pub enum PacketType {
  HANDSHAKE,
  FRAME,
  CLOSE,
  PING,
  PONG
}

impl PacketType {
  pub fn from_u32(value: u32) -> Self {
    match value {
      0 => PacketType::HANDSHAKE,
      1 => PacketType::FRAME,
      2 => PacketType::CLOSE,
      3 => PacketType::PING,
      4 => PacketType::PONG,
      _ => panic!("Invalid packet type: {}", value)
    }
  }
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Handshake {
  pub v: u32,
  pub client_id: String
}