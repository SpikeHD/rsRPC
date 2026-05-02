use interprocess::local_socket::traits::{Listener as _, Stream as _};
use interprocess::local_socket::{
  GenericFilePath, Listener, ListenerNonblockingMode, ListenerOptions, Stream, ToFsName,
};
use std::env;
use std::io::ErrorKind;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::cmd::ActivityCmd;
use crate::log;

use super::ipc_utils::{handle_stream, IpcFacilitator};

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
  } else if !temp.is_empty() {
    temp
  } else {
    "/tmp".to_string()
  };

  // Append a / to the temp dir if it doesn't have one
  let tmp_dir = if tmp_dir.ends_with('/') {
    tmp_dir
  } else {
    format!("{tmp_dir}/")
  };

  format!("{tmp_dir}discord-ipc")
}

struct BoundListener {
  socket: Listener,
  path: String,
}

impl Drop for BoundListener {
  fn drop(&mut self) {
    log!("[IPC] Cleaning up socket: {}", self.path);
    let _ = std::fs::remove_file(&self.path);
  }
}

#[derive(Clone)]
pub struct IpcConnector {
  socket: Arc<Mutex<BoundListener>>,
  did_handshake: bool,
  pub client_id: String,
  pub pid: u64,
  pub nonce: String,

  event_sender: mpsc::Sender<ActivityCmd>,
}

impl IpcFacilitator for IpcConnector {
  fn handshake(&self) -> bool {
    self.did_handshake
  }

  fn set_handshake(&mut self, handshake: bool) {
    self.did_handshake = handshake;
  }

  fn client_id(&self) -> String {
    self.client_id.clone()
  }

  fn set_client_id(&mut self, client_id: String) {
    self.client_id = client_id;
  }

  fn pid(&self) -> u64 {
    self.pid
  }

  fn set_pid(&mut self, pid: u64) {
    self.pid = pid;
  }

  fn nonce(&self) -> String {
    self.nonce.clone()
  }

  fn set_nonce(&mut self, nonce: String) {
    self.nonce = nonce;
  }

  fn recreate_socket(&mut self) {
    // Delete the socket, then create a new one
    let (socket, path) = Self::create_socket(None);
    *self.socket.lock().unwrap() = BoundListener { socket, path };
  }

  /**
   * Create a new thread that will recieve messages from the socket
   */
  fn start(&mut self) {
    let weak_socket = Arc::downgrade(&self.socket);
    let event_sender = self.event_sender.clone();
    let client_id = self.client_id.clone();
    let pid = self.pid;
    let nonce = self.nonce.clone();
    let did_handshake = self.did_handshake;

    thread::spawn(move || {
      if let Some(socket_arc) = weak_socket.upgrade() {
        let socket_guard = socket_arc.lock().unwrap();
        if let Err(err) = socket_guard
          .socket
          .set_nonblocking(ListenerNonblockingMode::Accept)
        {
          log!("[IPC] Failed to set socket to non-blocking: {}", err);
          return;
        }
      }

      loop {
        let socket_arc = match weak_socket.upgrade() {
          Some(arc) => arc,
          None => break,
        };

        let stream = {
          let socket_guard = socket_arc.lock().unwrap();
          socket_guard.socket.accept()
        };

        match stream {
          Ok(mut stream) => {
            log!("[IPC] Incoming stream...");

            let mut clone = IpcConnector {
              socket: socket_arc.clone(),
              did_handshake,
              client_id: client_id.clone(),
              pid,
              nonce: nonce.clone(),
              event_sender: event_sender.clone(),
            };
            thread::spawn(move || handle_stream(&mut clone, &mut stream));
          }
          Err(err) if err.kind() == ErrorKind::WouldBlock => {
            thread::sleep(Duration::from_millis(50));
          }
          Err(err) => {
            log!("[IPC] Error: {}", err);
            break;
          }
        }
      }
    });
  }

  fn event_sender(&mut self) -> &mut mpsc::Sender<ActivityCmd> {
    &mut self.event_sender
  }
}

impl IpcConnector {
  /**
   * Create a socket and return a new IpcConnector
   */
  pub fn new(event_sender: mpsc::Sender<ActivityCmd>) -> Self {
    let (socket, path) = Self::create_socket(None);

    Self {
      socket: Arc::new(Mutex::new(BoundListener { socket, path })),
      did_handshake: false,
      client_id: "".to_string(),
      pid: 0,
      nonce: "".to_string(),
      event_sender,
    }
  }

  /**
   * ACTUALLY create a socket, and return the handle
   */
  fn create_socket(tries: Option<u8>) -> (Listener, String) {
    let socket_path = get_socket_path();
    let tries = tries.unwrap_or(0);
    let socket_path = format!("{socket_path}-{tries}");

    log!("[IPC] Creating socket: {}", socket_path);

    let name = socket_path.clone().to_fs_name::<GenericFilePath>().unwrap();
    let listener_options = ListenerOptions::new().name(name.clone());

    let socket = match listener_options.create_sync() {
      Ok(socket) => socket,
      Err(err) => {
        if err.kind() == ErrorKind::AddrInUse {
          log!(
            "[IPC] Socket {} already in use, checking if stale...",
            socket_path
          );
          match Stream::connect(name) {
            Ok(_) => {
              log!("[IPC] Socket {} is in use by another process", socket_path);
            }
            Err(_) => {
              log!(
                "[IPC] Socket {} is stale, removing and retrying...",
                socket_path
              );
              let _ = std::fs::remove_file(&socket_path);
              let listener_options = ListenerOptions::new()
                .name(socket_path.clone().to_fs_name::<GenericFilePath>().unwrap());
              if let Ok(socket) = listener_options.create_sync() {
                log!(
                  "[IPC] Created IPC socket after cleaning stale: {}",
                  socket_path
                );
                return (socket, socket_path);
              }
            }
          }
        }

        log!("[IPC] Failed to create IPC socket: {}", err);

        if tries < 9 {
          return Self::create_socket(Some(tries + 1));
        } else {
          panic!("[IPC] Failed to create socket: {}", err);
        }
      }
    };

    log!("[IPC] Created IPC socket: {}", socket_path);

    (socket, socket_path)
  }
}
