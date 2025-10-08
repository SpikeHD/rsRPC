use clap::{command, Parser};
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

pub fn main() {
  // When running as a binary, enable logs
  std::env::set_var("RSRPC_LOGS_ENABLED", "1");

  let args = Args::parse();
  let config = RPCConfig {
    enable_process_scanner: !args.no_process_scan,
    ..Default::default()
  };
  let mut client = if args.no_process_scan {
    rsrpc::RPCServer::from_json_str("{}", config).expect("Failed to create RPCServer")
  } else if let Some(file) = args.detectable_file {
    rsrpc::RPCServer::from_file(file, config).expect("Failed to create RPCServer")
  } else {
    let detectable = reqwest::blocking::get("https://discord.com/api/v9/applications/detectable");
    rsrpc::RPCServer::from_json_str(detectable.unwrap().text().unwrap(), config)
      .expect("Failed to create RPCServer")
  };

  // Starts the other threads (process detector, client connector, etc)
  client.start();

  // let 'er run forever
  loop {
    std::thread::park();
    std::thread::sleep(std::time::Duration::from_secs(1));
  }
}
