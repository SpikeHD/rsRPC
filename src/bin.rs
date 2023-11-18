use rsrpc;
use std::path::PathBuf;

#[cfg(feature = "binary")]
pub fn main() {
  use clap::{command, Parser};
  use rsrpc::detection::{DetectableActivity, Executable};
  use std::sync::{Arc, Mutex};

  #[derive(Parser, Debug)]
  #[command(author, version, about, long_about = None)]
  struct Args {
    #[arg(short, long, default_value = "false")]
    detectable_file: PathBuf,
  }

  let args = Args::parse();

  // Create new client and stuff
  let client = Arc::new(Mutex::new(
    rsrpc::RPCServer::from_file(args.detectable_file).expect("Failed to create RPCServer"),
  ));

  // We can use this cloned Arc of the client to append new detectables
  let append_client = client.clone();

  // In a seperate thread, append a new detectable game
  std::thread::spawn(move || {
    let new_game: DetectableActivity = serde_json::from_str(r#"
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
      "summary": "",
      "type": 1
    }"#).unwrap();

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

  // When running as a binary, enable logs
  std::env::set_var("RSRPC_LOGS_ENABLED", "1");

  // Starts the other threads (process detector, client connector, etc)
  client.lock().unwrap().start();

  // let 'er run forever
  loop {}
}

#[cfg(not(feature = "binary"))]
pub fn main() {
  println!("This binary was not compiled with the binary feature enabled.");
  println!("Please compile with \"--features binary\" to enable the binary.");
}
