use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;

static LOGS_ENABLED: AtomicBool = AtomicBool::new(false);
static LOGS_INIT: Once = Once::new();

pub fn log(message: impl AsRef<str>) {
  LOGS_INIT.call_once(|| {
    if std::env::var("RSRPC_LOGS_ENABLED").unwrap_or_else(|_| "0".to_string()) == "1" {
      LOGS_ENABLED.store(true, Ordering::Relaxed);
    }
  });

  if LOGS_ENABLED.load(Ordering::Relaxed) {
    println!(
      "[{}] {}",
      chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
      message.as_ref()
    );
  }
}

#[macro_export]
macro_rules! log {
  ($($arg:tt)*) => {
    $crate::logger::log(format!($($arg)*))
  };
}
