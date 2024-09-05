use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct ActivityCmd {
  pub application_id: Option<String>,
  pub cmd: String,
  pub args: Option<ActivityCmdArgs>,
  pub data: Option<HashMap<String, String>>,
  pub evt: Option<String>,
  pub nonce: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ActivityCmdArgs {
  pub pid: Option<u64>,
  pub activity: Option<Activity>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Activity {
  pub state: Option<String>,
  pub details: Option<String>,
  pub timestamps: Option<Timestamps>,
  pub assets: Option<Assets>,
  pub buttons: Option<Vec<Button>>,
  pub instance: Option<bool>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Timestamps {
  pub start: i64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Assets {
  #[serde(rename = "large_image")]
  pub large_image: Option<String>,
  #[serde(rename = "large_text")]
  pub large_text: Option<String>,
  #[serde(rename = "small_image")]
  pub small_image: Option<String>,
  #[serde(rename = "small_text")]
  pub small_text: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Button {
  pub label: String,
  pub url: String,
}
