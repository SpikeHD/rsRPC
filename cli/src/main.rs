use clap::{command, Parser};
use rsrpc::RPCConfig;
use std::path::PathBuf;

pub fn main() {
  #[derive(Parser, Debug)]
  #[command(author, version, about, long_about = None)]
  struct Args {
    #[arg(short, long)]
    detectable_file: Option<PathBuf>,
  }

  let args = Args::parse();
  let mut client = if let Some(file) = args.detectable_file {
    rsrpc::RPCServer::from_file(file, RPCConfig::default()).expect("Failed to create RPCServer")
  } else {
    let detectable = reqwest::blocking::get("https://discord.com/api/v9/applications/detectable");
    rsrpc::RPCServer::from_json_str(detectable.unwrap().text().unwrap(), RPCConfig::default())
      .expect("Failed to create RPCServer")
  };

  // When running as a binary, enable logs
  std::env::set_var("RSRPC_LOGS_ENABLED", "1");

  // Starts the other threads (process detector, client connector, etc)
  client.start();

  // let 'er run forever
  loop {
    std::thread::sleep(std::time::Duration::from_millis(10));
  }
}
