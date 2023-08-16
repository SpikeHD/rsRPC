pub struct BaseServer {
  on_connect: fn(),
  on_disconnect: fn(),
  on_message: fn(),
}

impl BaseServer {
  pub fn new() -> Self {
    Self {
      on_connect: || {},
      on_disconnect: || {},
      on_message: || {},
    }
  }
}