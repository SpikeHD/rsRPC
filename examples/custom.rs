use rsrpc::{detection::DetectableActivity, RPCConfig};
use std::sync::{Arc, Mutex};

pub fn main() {
  // When running as a binary, enable logs
  std::env::set_var("RSRPC_LOGS_ENABLED", "1");

  // Create new client and stuff
  let client = Arc::new(Mutex::new(
    rsrpc::RPCServer::from_json_str("{}", RPCConfig::default())
      .expect("Failed to create RPCServer"),
  ));

  // We can use this cloned Arc of the client to append new detectables
  let append_client = client.clone();

  // In a seperate thread, append a new detectable game
  std::thread::spawn(move || {
    let new_game: DetectableActivity = serde_json::from_str(
      r#"
    {
      "bot_public": true,
      "bot_require_code_grant": false,
      "description": "",
      "executables": [
        {
          "is_launcher": false,
          "name": "my_awesome_game.exe",
          "os": "win32"
        }
      ],
      "flags": 0,
      "hook": true,
      "id": "1337",
      "name": "Awesome Game!!",
      "type": 1
    }"#,
    )
    .unwrap();

    // This function takes a vec, as you might want to add more than one game (say, if you stored configurable lists of games in a seperate file)
    append_client
      .lock()
      .unwrap()
      .append_detectables(vec![new_game]);

    // Remove a custom game via its name
    append_client
      .lock()
      .unwrap()
      .remove_detectable_by_name("Awesome Game!!".to_string());
  });

  // Starts the other threads (process detector, client connector, etc)
  client.lock().unwrap().start();

  // let 'er run forever
  loop {
    std::thread::sleep(std::time::Duration::from_millis(10));
  }
}
