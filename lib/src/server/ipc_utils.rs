use std::{
  io::{Read, Write},
  sync::mpsc,
};

use interprocess::local_socket::Stream;
use serde_json::Value;

use crate::{
  cmd::{ActivityCmd, ActivityCmdArgs},
  log,
  server::utils,
};

pub trait IpcFacilitator {
  fn handshake(&self) -> bool;
  fn set_handshake(&mut self, handshake: bool);

  fn client_id(&self) -> String;
  fn set_client_id(&mut self, client_id: String);

  fn pid(&self) -> u64;
  fn set_pid(&mut self, pid: u64);

  fn nonce(&self) -> String;
  fn set_nonce(&mut self, nonce: String);

  fn recreate_socket(&mut self);

  fn start(&mut self);

  fn event_sender(&mut self) -> &mut mpsc::Sender<ActivityCmd>;
}

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

#[allow(clippy::result_large_err)]
pub fn send_empty(
  event_sender: &mut mpsc::Sender<ActivityCmd>,
  pid: u64,
) -> Result<(), mpsc::SendError<ActivityCmd>> {
  log!("[IPC] Sending empty activity");

  let activity = ActivityCmd {
    args: Some(ActivityCmdArgs {
      activity: None,
      code: None,
      pid: Some(pid),
    }),
    ..ActivityCmd::empty()
  };
  event_sender.send(activity)
}

pub fn handle_stream(ipc: &mut dyn IpcFacilitator, stream: &mut Stream) {
  loop {
    let current_pid = ipc.pid();
    // Read into buffer
    let mut buffer = std::io::BufReader::new(&mut *stream);

    // Read the packet type and size
    let mut packet_type = [0; 4];
    let mut data_size = [0; 4];

    match buffer.by_ref().take(4).read_exact(&mut packet_type) {
      Ok(_) => (),
      Err(err) => {
        log!(
          "[IPC] Error reading packet type: {}, socket likely closed",
          err
        );

        // Send empty activity
        send_empty(ipc.event_sender(), current_pid)
          .unwrap_or_else(|e| log!("[IPC] Error sending empty activity: {}", e));
        break;
      }
    }

    match buffer.by_ref().take(4).read_exact(&mut data_size) {
      Ok(_) => (),
      Err(err) => {
        log!("[IPC] Error reading data size: {}", err);

        // Send empty activity
        send_empty(ipc.event_sender(), current_pid)
          .unwrap_or_else(|e| log!("[IPC] Error sending empty activity: {}", e));
        break;
      }
    }

    // Convert the rest of the buffer to a string
    let mut message = String::new();

    match buffer
      .by_ref()
      .take(u32::from_le_bytes(data_size) as u64)
      .read_to_string(&mut message)
    {
      Ok(_) => (),
      Err(err) => {
        log!("[IPC] Error reading data: {}", err);
        break;
      }
    }

    let r_type = PacketType::from_u32(u32::from_le_bytes(packet_type));

    log!("[IPC] Recieved message: {}", message);

    match r_type {
      PacketType::Handshake => {
        log!("[IPC] Recieved handshake");
        let Ok(data) = serde_json::from_str::<Handshake>(&message) else {
          log!("[IPC] Error parsing handshake");
          continue;
        };

        if data.v != 1 {
          log!("[IPC] Invalid version: {}", data.v);
          continue;
        }

        ipc.set_handshake(true);
        ipc.set_client_id(data.client_id);

        // Send CONNECTION_RESPONSE
        let resp = encode(PacketType::Frame, utils::CONNECTION_REPONSE.to_string());

        match stream.write_all(&resp) {
          Ok(_) => (),
          Err(err) => log!("[IPC] Error sending connection response: {}", err),
        }
      }
      PacketType::Frame => {
        if !ipc.handshake() {
          log!("[IPC] Did not handshake yet, ignoring frame");
          continue;
        }

        let mut activity_cmd = match serde_json::from_str::<ActivityCmd>(&message) {
          Ok(cmd) => cmd,
          Err(err) => {
            log!("[IPC] Error parsing activity command: {}", err);
            continue;
          }
        };

        let args = match activity_cmd.args {
          Some(ref args) => args,
          None => {
            log!("[IPC] Invalid activity command, skipping");

            // Send empty activity
            send_empty(ipc.event_sender(), current_pid)
              .unwrap_or_else(|e| log!("[IPC] Error sending empty activity: {}", e));
            continue;
          }
        };

        activity_cmd.application_id = Some(ipc.client_id());

        ipc.set_pid(args.pid.unwrap_or_default());
        ipc.set_nonce(activity_cmd.nonce.to_string());

        match ipc.event_sender().send(activity_cmd) {
          Ok(_) => (),
          Err(err) => log!("[IPC] Error sending activity command: {}", err),
        }
      }
      PacketType::Close => {
        log!("[IPC] Recieved close");

        // Send message with an empty activity
        let activity_cmd = ActivityCmd {
          application_id: Some(ipc.client_id()),
          cmd: "SET_ACTIVITY".to_string(),
          data: None,
          evt: None,
          args: Some(ActivityCmdArgs {
            pid: Some(ipc.pid()),
            activity: None,
            code: None,
          }),
          nonce: Value::String(ipc.nonce()),
        };

        match ipc.event_sender().send(activity_cmd) {
          Ok(_) => (),
          Err(err) => log!("[IPC] Error sending activity command: {}", err),
        }

        // reset values
        ipc.set_handshake(false);
        ipc.set_client_id("".to_string());
        ipc.set_pid(0);

        ipc.recreate_socket();

        break;
      }
      PacketType::Ping => {
        log!("[IPC] Recieved ping");

        // Send a pong
        let resp = encode(PacketType::Pong, message);

        match stream.write_all(&resp) {
          Ok(_) => (),
          Err(err) => log!("[IPC] Error sending pong: {}", err),
        };
      }
      PacketType::Pong => {
        log!("[IPC] Recieved pong");
      }
    }
  }
}
