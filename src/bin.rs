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
  let client =
    Arc::new(Mutex::new(rsrpc::RPCServer::from_file(args.detectable_file).expect("Failed to create RPCServer")));

  // We can use this cloned Arc of the client to append new detectables
  let append_client = client.clone();

  // In a seperate thread, append a new detectable game
  std::thread::spawn(move || {
    let new_game = DetectableActivity {
      bot_public: None,
      bot_require_code_grant: None,
      cover_image: None,
      description: None,
      developers: None,
      executables: Some(vec![Executable {
        is_launcher: false,
        name: "awesome_game.exe".to_string(),
        os: "win32".to_string(),
        arguments: None,
      }]),
      flags: None,
      guild_id: None,
      hook: false,
      icon: None,
      id: "null".to_string(),
      name: "My Awesome Game!".to_string(),
      publishers: vec![],
      rpc_origins: vec![],
      splash: None,
      summary: "".to_string(),
      third_party_skus: vec![],
      type_field: None,
      verify_key: "".to_string(),
      primary_sku_id: None,
      slug: None,
      aliases: vec![],
      overlay: None,
      overlay_compatibility_hook: None,
      privacy_policy_url: None,
      terms_of_service_url: None,
      eula_id: None,
      deeplink_uri: None,
      tags: vec![],
      pid: None,
      timestamp: None,
    };

    // This function takes a vec, as you might want to add more than one game (say, if you stored configurable lists of games in a seperate file)
    append_client.lock().unwrap().append_detectables(vec![new_game]);
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
