pub fn log(message: impl AsRef<str>) {
  // If LOGS_ENABLED is 1, log the message with a timestamp
  if std::env::var("LOGS_ENABLED").unwrap_or("0".to_string()) == "1" {
    println!("[{}] {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), message.as_ref());
  }
}
