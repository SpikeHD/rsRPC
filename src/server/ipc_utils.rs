#[derive(Debug)]
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
      _ => PacketType::Frame,
    }
  }
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Handshake {
  pub v: u32,
  pub client_id: String,
}

pub fn encode(r_type: PacketType, data: String) -> Vec<u8> {
  let mut buffer: Vec<u8> = Vec::new();

  // Write the packet type
  buffer.extend_from_slice(&u32::to_le_bytes(r_type as u32));

  // Write the data size
  buffer.extend_from_slice(&u32::to_le_bytes(data.len() as u32));

  // Write the data
  buffer.extend_from_slice(data.as_bytes());

  buffer
}