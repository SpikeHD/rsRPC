use interprocess::local_socket::traits::ListenerExt;
use interprocess::local_socket::{Listener, ListenerOptions, ToFsName};
use interprocess::os::windows::local_socket::NamedPipe;
use std::io::{Read, Write};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use crate::cmd::{ActivityCmd, ActivityCmdArgs};
use crate::log;
use crate::server::utils;

use super::ipc_utils::encode;
use super::ipc_utils::Handshake;
use super::ipc_utils::PacketType;

#[derive(Clone)]
pub struct IpcConnector {
  socket: Arc<Mutex<Listener>>,
  did_handshake: bool,
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
    Self {
      socket: Arc::new(Mutex::new(Self::create_socket(None))),
      did_handshake: false,
      client_id: "".to_string(),
      pid: 0,
      nonce: "".to_string(),
      event_sender,
    }
  }

  fn create_socket(tries: Option<u8>) -> Listener {
    // Define the path to the named pipe
    let pipe_path = r"\\.\pipe\discord-ipc";

    // Append tried number to name if applicable
    let pipe_path = match tries {
      Some(tries) => format!("{}-{}", pipe_path, tries),
      None => format!("{}-{}", pipe_path, 0),
    };

    let listener =
      ListenerOptions::new().name(pipe_path.clone().to_fs_name::<NamedPipe>().unwrap());

    let socket = match listener.create_sync() {
      Ok(socket) => socket,
      Err(err) => {
        log!("[IPC] Failed to create IPC socket: {}", err);

        if tries.unwrap_or(0) < 9 {
          return Self::create_socket(Some(tries.unwrap_or(0) + 1));
        } else {
          panic!("[IPC] Failed to create socket: {}", err);
        }
      }
    };

    log!("[IPC] Created IPC socket: {}", pipe_path);

    socket
  }

  pub fn send_empty(event_sender: &mut mpsc::Sender<ActivityCmd>) -> Result<(), mpsc::SendError<ActivityCmd>> {
    let activity = ActivityCmd {
      args: Some(ActivityCmdArgs {
        activity: None,
        code: None,
        pid: None,
      }),
      ..ActivityCmd::empty()
    };
    event_sender.send(activity)
  }

  /**
   * Create a new thread that will recieve messages from the socket
   */
  pub fn start(&mut self) {
    let mut clone = self.clone();

    std::thread::spawn(move || {
      let socket = clone.socket.lock().unwrap();

      for stream in socket.incoming() {
        // Little baby delay to keep things smooth
        std::thread::sleep(std::time::Duration::from_millis(5));

        match stream {
          Ok(stream) => {
            // Read into buffer
            let mut buffer = std::io::BufReader::new(&stream);

            loop {
              // Read the packet type and size
              let mut packet_type = [0; 4];
              let mut data_size = [0; 4];

              match buffer.by_ref().take(4).read_exact(&mut packet_type) {
                Ok(_) => (),
                Err(err) => {
                  log!("[IPC] Error reading packet type: {}, socket likely closed", err);

                  // Send empty activity
                  Self::send_empty(&mut clone.event_sender)
                    .unwrap_or_else(|e| log!("[IPC] Error sending empty activity: {}", e));
                  break;
                }
              }

              match buffer.by_ref().take(4).read_exact(&mut data_size) {
                Ok(_) => (),
                Err(err) => {
                  log!("[IPC] Error reading data size: {}", err);
                  
                  // Send empty activity
                  Self::send_empty(&mut clone.event_sender)
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
                  
                  // Send empty activity
                  Self::send_empty(&mut clone.event_sender)
                    .unwrap_or_else(|e| log!("[IPC] Error sending empty activity: {}", e));
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

                  clone.did_handshake = true;
                  clone.client_id = data.client_id;

                  // Send CONNECTION_RESPONSE
                  let resp = encode(PacketType::Frame, utils::CONNECTION_REPONSE.to_string());

                  match buffer.get_mut().write_all(&resp) {
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
                    
                    // Send empty activity
                    Self::send_empty(&mut clone.event_sender)
                      .unwrap_or_else(|e| log!("[IPC] Error sending empty activity: {}", e));
                    continue;
                  };

                  let args = match activity_cmd.args {
                    Some(ref args) => args,
                    None => {
                      log!("[IPC] Invalid activity command, skipping");
                      
                      // Send empty activity
                      Self::send_empty(&mut clone.event_sender)
                        .unwrap_or_else(|e| log!("[IPC] Error sending empty activity: {}", e));
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

                  // Drop and recreate socket
                  let mut socket = clone.socket.lock().unwrap();
                  *socket = Self::create_socket(None);

                  break;
                }
                PacketType::Ping => {
                  log!("[IPC] Recieved ping");

                  // Send a pong
                  let resp = encode(PacketType::Pong, message);

                  match buffer.get_mut().write_all(&resp) {
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
          Err(err) => {
            log!("[IPC] Error: {}", err);
            break;
          }
        }
      }
    });
  }
}
