use rsrpc::RPCConfig;

pub fn main() {
  // When running as a binary, enable logs
  std::env::set_var("RSRPC_LOGS_ENABLED", "1");

  // Create new client and stuff
  let mut client = rsrpc::RPCServer::from_json_str(
    // in order for the process scanner to actually scan, we need to provide a list of detectable games (even if just one)
    r#"
    [{
      "bot_public": true,
      "bot_require_code_grant": false,
      "description": "",
      "executables": [
        {
          "is_launcher": false,
          "name": "game_that_doesnt_exist.exe",
          "os": "win32"
        }
      ],
      "flags": 0,
      "hook": true,
      "id": "0",
      "name": "X",
      "type": 1
    }]
  "#,
    RPCConfig::default(),
  )
  .expect("Failed to create RPCServer");

  client.on_process_scan_complete(move |state| {
    if state.obs_open {
      println!("Streamer mode should be toggled");
    } else {
      println!("Streamer mode should NOT be toggled");
    }
  });

  // Starts the other threads (process detector, client connector, etc)
  client.start();

  // let 'er run forever
  loop {
    std::thread::sleep(std::time::Duration::from_millis(10));
  }
}
