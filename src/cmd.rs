use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ActivityCmd {
  pub application_id: Option<String>,
  pub cmd: String,
  pub args: Option<ActivityCmdArgs>,
  pub data: Option<HashMap<String, String>>,
  pub evt: Option<String>,
  pub nonce: String,
}

impl ActivityCmd {
  pub fn empty() -> Self {
    Self {
      application_id: None,
      cmd: "".to_string(),
      args: None,
      data: None,
      evt: None,
      nonce: "".to_string(),
    }
  }

  pub fn fix_timestamps(&mut self) {
    if let Some(timestamps) = self.args
      .as_mut()
      .and_then(|args| args.activity.as_mut())
      .and_then(|activity| activity.timestamps.as_mut())
    {
      // convert starting timestamp
      if let Some(start) = timestamps.start.as_mut() {
        // convert timestamp
        let s = chrono::DateTime::from_timestamp(start.0, 0);
        if let Some(s) = s {
          *start = TimeoutValue(s.timestamp_millis());
        }
      }

      // convert ending timestamp
      if let Some(end) = timestamps.end.as_mut() {
        // convert timestamp
        let s = chrono::DateTime::from_timestamp(end.0, 0);
        if let Some(s) = s {
          *end = TimeoutValue(s.timestamp_millis());
        }
      }
    }
  }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ActivityCmdArgs {
  pub pid: Option<u64>,
  pub activity: Option<Activity>,
  // For INVITE_BROWSER
  pub code: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Activity {
  #[serde(rename = "type")]
  pub r#type: Option<i64>,
  pub state: Option<String>,
  pub details: Option<String>,
  pub timestamps: Option<Timestamps>,
  pub assets: Option<Assets>,
  pub buttons: Option<Vec<Button>>,
  pub instance: Option<bool>,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct TimeoutValue(i64);

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Timestamps {
  #[serde(default)]
  pub start: Option<TimeoutValue>,
  #[serde(default)]
  pub end: Option<TimeoutValue>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
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

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Button {
  pub label: String,
  pub url: String,
}
