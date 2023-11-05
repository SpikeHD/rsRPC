use serde::Deserialize;

#[derive(Deserialize)]
pub struct ActivityCmd {
  pub cmd: String,
  pub args: ActivityCmdArgs,
  pub nonce: String,
}

#[derive(Deserialize)]
pub struct ActivityCmdArgs {
  pub pid: u64,
  pub activity: Option<Activity>,
}

#[derive(Deserialize)]
pub struct Activity {
  pub state: String,
  pub details: String,
  pub timestamps: Timestamps,
  pub assets: Assets,
  pub buttons: Vec<Button>,
  pub instance: bool,
}

#[derive(Deserialize)]
pub struct Timestamps {
  pub start: i64,
}

#[derive(Deserialize)]
pub struct Assets {
  #[serde(rename = "large_image")]
  pub large_image: String,
  #[serde(rename = "large_text")]
  pub large_text: String,
  #[serde(rename = "small_image")]
  pub small_image: String,
  #[serde(rename = "small_text")]
  pub small_text: String,
}

#[derive(Deserialize)]
pub struct Button {
  pub label: String,
  pub url: String,
}
