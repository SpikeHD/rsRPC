use crate::cmd::{ActivityCmd, ActivityCmdArgs};
use crate::log;
use crate::server::ipc_utils::Handshake;
use crate::server::utils;
use std::env;
use std::io::{Read, Write};
use std::os::unix::net::UnixListener;
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use super::ipc_utils::PacketType;

fn get_socket_path() -> String {
  let xdg_runtime_dir = env::var("XDG_RUNTIME_DIR").unwrap_or_default();
  let tmpdir = env::var("TMPDIR").unwrap_or_default();
  let tmp = env::var("TMP").unwrap_or_default();
  let temp = env::var("TEMP").unwrap_or_default();
  let tmp_dir = if !xdg_runtime_dir.is_empty() {
    xdg_runtime_dir
  } else if !tmpdir.is_empty() {
    tmpdir
  } else if !tmp.is_empty() {
    tmp
  } else {
    temp
  };

  // Append a / to the temp dir if it doesn't have one
  let tmp_dir = if tmp_dir.ends_with('/') {
    tmp_dir
  } else {
    format!("{}/", tmp_dir)
  };

  format!("{}discord-ipc", tmp_dir)
}

#[derive(Clone)]
pub struct IpcConnector {
  socket: Arc<Mutex<UnixListener>>,
  pub did_handshake: bool,
  pub client_id: String,
  pub pid: u64,
  pub nonce: String,

  event_sender: mpsc::Sender<ActivityCmd>,
}

impl IpcConnector {
  /**
   * Create a socket and return a new IpcConnector
   */
  pub fn new(event_sender: mpsc::Sender<ActivityCmd>) -> Self {
    let socket = Self::create_socket(None);

    Self {
      socket: Arc::new(Mutex::new(socket)),
      did_handshake: false,
      client_id: "".to_string(),
      pid: 0,
      nonce: "".to_string(),
      event_sender,
    }
  }

  /**
   * Close and delete the socket
   */
  pub fn _close(&mut self) {
    let socket_addr = self.socket.lock().unwrap().local_addr().unwrap();
    let path = socket_addr
      .as_pathname()
      .unwrap_or(std::path::Path::new(""));
    std::fs::remove_file(path).unwrap_or_default();
  }

  /**
   * ACTUALLY create a socket, and return the handle
   */
  fn create_socket(tries: Option<u8>) -> UnixListener {
    let socket_path = get_socket_path();
    let tries = tries.unwrap_or(0);

    log!("[IPC] Creating socket: {}-{}", socket_path, tries);

    let socket = UnixListener::bind(format!("{}-{}", socket_path, tries));

    match socket {
      Ok(socket) => socket,
      Err(err) => {
        if tries >= 10 {
          log!("[IPC] Could not create IPC socket after 10 tries: {}", err);
          panic!("Could not create IPC socket after 10 tries");
        }

        std::thread::sleep(Duration::from_millis(500));

        Self::create_socket(Some(tries + 1))
      }
    }
  }

  pub fn start(&mut self) {
    let mut clone = self.clone();

    std::thread::spawn(move || {
      let socket = clone.socket.lock().unwrap();

      // Forever recieve messages from the socket
      for stream in socket.incoming() {
        // Little baby delay to keep things smooth
        std::thread::sleep(std::time::Duration::from_millis(5));

        match stream {
          Ok(mut stream) => {
            // Read into buffer
            let mut buffer = std::io::BufReader::new(&stream);

            // Read the packet type and size
            let mut packet_type = [0; 4];
            let mut data_size = [0; 4];

            buffer
              .by_ref()
              .take(8)
              .read_exact(&mut packet_type)
              .expect("Could not read packet type");

            buffer
              .by_ref()
              .take(8)
              .read_exact(&mut data_size)
              .expect("Could not read data size");

            // Conver the rest of the buffer to a string
            let mut message = String::new();

            buffer
              .by_ref()
              .take(u32::from_le_bytes(data_size) as u64)
              .read_to_string(&mut message)
              .expect("Could not read data");

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
                  panic!("Invalid version: {}", data.v);
                }

                clone.did_handshake = true;
                clone.client_id = data.client_id;

                // Send CONNECTION_RESPONSE
                let resp = encode(PacketType::Frame, utils::CONNECTION_REPONSE.to_string());

                match stream.write_all(&resp) {
                  Ok(_) => (),
                  Err(err) => log!("[IPC] Error sending connection response: {}", err),
                }
              }
              PacketType::Frame => {
                if !clone.did_handshake {
                  log!("[IPC] Did not handshake yet, ignoring frame");
                  continue;
                }

                let Ok(mut activity_cmd) = serde_json::from_str::<ActivityCmd>(&message) else {
                  log!("[IPC] Error parsing activity command");
                  continue;
                };

                let args = match activity_cmd.args {
                  Some(ref args) => args,
                  None => {
                    log!("[IPC] Invalid activity command, skipping");
                    continue;
                  }
                };

                activity_cmd.application_id = Some(clone.client_id.clone());

                clone.pid = args.pid.unwrap_or_default();
                clone.nonce.clone_from(&activity_cmd.nonce);

                match clone.event_sender.send(activity_cmd) {
                  Ok(_) => (),
                  Err(err) => log!("[IPC] Error sending activity command: {}", err),
                }
              }
              PacketType::Close => {
                log!("[IPC] Recieved close");

                // Send message with an empty activity
                let activity_cmd = ActivityCmd {
                  application_id: Some(clone.client_id.clone()),
                  cmd: "SET_ACTIVITY".to_string(),
                  data: None,
                  evt: None,
                  args: Some(ActivityCmdArgs {
                    pid: Some(clone.pid),
                    activity: None,
                    code: None,
                  }),
                  nonce: clone.nonce.clone(),
                };

                match clone.event_sender.send(activity_cmd) {
                  Ok(_) => (),
                  Err(err) => log!("[IPC] Error sending activity command: {}", err),
                }

                // reset values
                clone.did_handshake = false;
                clone.client_id = "".to_string();
                clone.pid = 0;

                // Delete and recreate socket
                let mut socket = clone.socket.lock().unwrap();

                let socket_addr = socket.local_addr().unwrap();
                let path = socket_addr
                  .as_pathname()
                  .unwrap_or(std::path::Path::new(""));

                std::fs::remove_file(path).unwrap_or_default();

                *socket = Self::create_socket(None);
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
          Err(err) => {
            log!("[IPC] Error: {}", err);
            break;
          }
        }
      }
    });

    fn encode(r_type: PacketType, data: String) -> Vec<u8> {
      let mut buffer: Vec<u8> = Vec::new();

      // Write the packet type
      buffer.extend_from_slice(&u32::to_le_bytes(r_type as u32));

      // Write the data size
      buffer.extend_from_slice(&u32::to_le_bytes(data.len() as u32));

      // Write the data
      buffer.extend_from_slice(data.as_bytes());

      buffer
    }
  }
}
