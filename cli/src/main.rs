#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use clap::Parser;
use rsrpc::RPCConfig;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
  #[arg(short, long)]
  detectable_file: Option<PathBuf>,
  #[arg(short, long)]
  no_process_scan: bool,
}

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
  // When running as a binary, enable logs
  std::env::set_var("RSRPC_LOGS_ENABLED", "1");

  let args = Args::parse();
  let config = RPCConfig {
    enable_process_scanner: !args.no_process_scan,
    ..Default::default()
  };
  let mut client = if args.no_process_scan {
    rsrpc::RPCServer::from_json_str("[]", config).expect("Failed to create RPCServer")
  } else if let Some(file) = args.detectable_file {
    rsrpc::RPCServer::from_file(file, config).expect("Failed to create RPCServer")
  } else {
    let detectable = ureq::get("https://discord.com/api/v9/applications/detectable")
      .call()?
      .into_body()
      .with_config()
      .limit(32 * 1024 * 1024)
      .read_to_string()?;
    rsrpc::RPCServer::from_json_str(detectable, config).expect("Failed to create RPCServer")
  };

  // Starts the other threads (process detector, client connector, etc)
  client.start();

  let (tx, rx) = std::sync::mpsc::channel();
  ctrlc::set_handler(move || {
    let _ = tx.send(());
  })
  .expect("Error setting Ctrl-C handler");

  println!("Press Ctrl+C to exit");
  let _ = rx.recv();

  println!("Shutting down...");
  drop(client);

  // giving them a bit so they can clean up (e.g. drop BoundListener)
  std::thread::sleep(std::time::Duration::from_millis(100));

  Ok(())
}
