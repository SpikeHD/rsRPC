use std::path::PathBuf;
use rsrpc;

#[cfg(feature = "binary")]
pub fn main() {
  use clap::{Parser, command};
  
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

  client.start();
}

#[cfg(not(feature = "binary"))]
pub fn main() {
  println!("This binary was not compiled with the binary feature enabled.");
  println!("Please compile with \"--features binary\" to enable the binary.");
}