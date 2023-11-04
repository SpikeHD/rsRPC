use rsrpc;
use std::path::PathBuf;

#[cfg(feature = "binary")]
pub fn main() {
  use clap::{command, Parser};

  #[derive(Parser, Debug)]
  #[command(author, version, about, long_about = None)]
  struct Args {
    #[arg(short, long, default_value = "false")]
    detectable_file: PathBuf,

    #[arg(short, long, default_value = None)]
    process_scan_ms: Option<String>,
  }

  let args = Args::parse();

  // Create new client and stuff
  let mut client = rsrpc::RPCServer::from_file(args.detectable_file);

  client.process_scan_ms = Some(100);

  if args.process_scan_ms.is_some() {
    client.process_scan_ms = Some(args.process_scan_ms.unwrap().parse::<u64>().unwrap());
  }

  // When running as a binary, enable logs
  std::env::set_var("RSRPC_LOGS_ENABLED", "1");

  client.start();
}

#[cfg(not(feature = "binary"))]
pub fn main() {
  println!("This binary was not compiled with the binary feature enabled.");
  println!("Please compile with \"--features binary\" to enable the binary.");
}
