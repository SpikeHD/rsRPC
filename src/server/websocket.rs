use std::{
  collections::HashMap,
  sync::{mpsc, Arc, Mutex},
};

use simple_websockets::{Event, EventHub, Message, Responder};

use crate::{
  cmd::{ActivityCmd, ActivityCmdArgs},
  log,
  server::utils::CONNECTION_REPONSE,
  url_params::get_url_params,
};

type ActivityResponder = (Option<ActivityCmd>, Responder);

#[derive(Clone)]
pub struct WebsocketConnector {
  server: Arc<Mutex<EventHub>>,
  pub clients: Arc<Mutex<HashMap<u64, ActivityResponder>>>,

  event_sender: mpsc::Sender<ActivityCmd>,
}

impl WebsocketConnector {
  pub fn new(event_sender: mpsc::Sender<ActivityCmd>) -> Self {
    // Try starting websocket server on ports 6463 - 6472
    for port in 6463..6472 {
      match simple_websockets::launch(port) {
        Ok(server) => {
          log!("[Websocket] Server started on port {}", port);
          return Self {
            server: Arc::new(Mutex::new(server)),
            clients: Arc::new(Mutex::new(HashMap::new())),
            event_sender,
          };
        }
        Err(_) => {
          log!("[Websocket] Failed to start server on port {}", port);
        }
      }
    }

    log!("[Websocket] Failed to start server on any port");
    std::process::exit(1);
  }

  pub fn start(&self) {
    let server = self.server.clone();
    let clients = self.clients.clone();
    let event_sender = self.event_sender.clone();

    std::thread::spawn(move || {
      let server = server.lock().unwrap();
      let mut clients = clients.lock().unwrap();

      loop {
        log!("[Websocket] Polling for events...");

        match server.poll_event() {
          Event::Connect(client_id, responder) => {
            let connection = responder.connection_details();
            let url_params = get_url_params(connection.uri.clone());
            let version = url_params.get("v").unwrap_or(&"0".to_string()).clone();
            let encoding = url_params
              .get("encoding")
              .unwrap_or(&"json".to_string())
              .clone();

            log!("[Websocket] Client {} connected", client_id);

            if version != "1" || encoding != "json" {
              log!("[Websocket] Invalid connection from client {}", client_id);
              continue;
            }

            responder.send(Message::Text(CONNECTION_REPONSE.to_string()));

            clients.insert(client_id, (None, responder));
          }
          Event::Disconnect(client_id) => {
            log!("[Websocket] Client {} disconnected", client_id);
            let responder = clients.remove(&client_id).unwrap();

            handle_disconnect(client_id, &event_sender, &responder);
          }
          Event::Message(client_id, message) => {
            log!(
              "[Websocket] Received message from client {}: {:?}",
              client_id,
              message
            );

            let responder = clients.get_mut(&client_id).unwrap();
            let message = match message {
              Message::Text(text) => text,
              _ => "".to_string(),
            };

            // If not ActivityCmd, ignore
            let event: ActivityCmd = match serde_json::from_str(&message) {
              Ok(event) => event,
              Err(e) => {
                log!("[Websocket] Invalid message from client {}", client_id);
                log!("[Websocket] Error: {}", e);
                continue;
              }
            };

            // If origin isn't a Discord URL, ignore
            let origin = responder.1.connection_details().headers.get("origin");

            if let Some(origin) = origin {
              let value = origin.to_str().unwrap_or_default();
              let valid = [
                "https://discord.com",
                "https://canary.discord.com",
                "https://ptb.discord.com",
              ];

              if !valid.contains(&value) {
                log!("[Websocket] Invalid origin from client {}", client_id);
                continue;
              }
            }

            match event.cmd.as_str() {
              "INVITE_BROWSER" => handle_invite(&event, &event_sender, &responder.1),
              "SET_ACTIVITY" => handle_set_activity(&event, &event_sender, responder),
              "DEEP_LINK" => {
                log!("[Websocket] Deep link unimplemented. PRs are open!");
              }
              _ => {
                log!("[Websocket] Unknown command: {}", event.cmd);
              }
            }
          }
        }
      }
    });
  }
}

fn event_args_as_hashmap(args: Option<ActivityCmdArgs>) -> HashMap<String, String> {
  // Serde serialize the args
  let args = match args {
    Some(args) => serde_json::to_string(&args).unwrap_or("".to_string()),
    None => "{}".to_string(),
  };

  // Re-deserialize the args as a hashmap
  serde_json::from_str::<HashMap<String, String>>(&args).unwrap_or_default()
}

fn handle_invite(
  event: &ActivityCmd,
  event_sender: &mpsc::Sender<ActivityCmd>,
  responder: &Responder,
) {
  // Let's just assume this went well I don't care
  let response = ActivityCmd {
    application_id: event.application_id.clone(),
    cmd: event.cmd.clone(),
    args: None,
    data: Some(event_args_as_hashmap(event.args.clone())),
    evt: None,
    nonce: event.nonce.clone(),
  };

  // Send the event away!
  event_sender.send(event.clone()).unwrap();

  // Respond
  responder.send(Message::Text(serde_json::to_string(&response).unwrap()));
}

fn handle_set_activity(
  event: &ActivityCmd,
  event_sender: &mpsc::Sender<ActivityCmd>,
  responder: &mut ActivityResponder,
) {
  // Set the last activity for the client
  responder.0 = Some(event.clone());

  event_sender.send(event.clone()).unwrap();
}

fn handle_disconnect(
  _client_id: u64,
  event_sender: &mpsc::Sender<ActivityCmd>,
  responder: &ActivityResponder,
) {
  if let Some(ref activity_cmd) = responder.0 {
    // Send empty activity
    let activity_cmd = ActivityCmd {
      application_id: activity_cmd.application_id.clone(),
      cmd: "SET_ACTIVITY".to_string(),
      data: None,
      evt: None,
      args: Some(ActivityCmdArgs {
        pid: Some(activity_cmd.args.as_ref().unwrap().pid.unwrap_or_default()),
        activity: None,
      }),
      nonce: activity_cmd.nonce.clone(),
    };

    event_sender.send(activity_cmd).unwrap();
  }
}
