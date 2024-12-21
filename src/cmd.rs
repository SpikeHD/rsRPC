use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::collections::HashMap;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ActivityPayload {
  pub activity: Option<Activity>,
  pub pid: Option<u64>,
  #[serde(rename = "socketId")]
  pub socket_id: Option<String>,
}

#[skip_serializing_none]
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
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
    if let Some(timestamps) = self
      .args
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

#[skip_serializing_none]
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ActivityCmdArgs {
  pub pid: Option<u64>,
  pub activity: Option<Activity>,
  // For INVITE_BROWSER
  pub code: Option<String>,
}

#[skip_serializing_none]
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Party {
  pub id: Option<String>,
  pub size: Option<Vec<u32>>,
}

#[skip_serializing_none]
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct Assets {
  pub large_image: Option<String>,
  pub large_text: Option<String>,
  pub small_image: Option<String>,
  pub small_text: Option<String>,
}

#[skip_serializing_none]
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Secrets {
  pub join: Option<String>,
}

#[skip_serializing_none]
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct Emoji {
  pub name: Option<String>,
  pub id: Option<String>,
  pub animated: Option<bool>,
}

// Important: https://docs.discord.sex/resources/presence#activity-metadata-object
#[skip_serializing_none]
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub struct Metadata {
  pub button_urls: Option<Vec<String>>,
  pub artist_ids: Option<Vec<String>>,
  pub album_id: Option<String>,
  pub context_uri: Option<String>,
}

#[skip_serializing_none]
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct Activity {
  pub id: Option<String>,
  pub name: Option<String>,
  pub r#type: Option<u32>,
  pub url: Option<String>,
  pub created_at: Option<u64>,
  pub session_id: Option<String>,
  pub platform: Option<String>,
  pub supported_platforms: Option<Vec<String>>,
  pub timestamps: Option<Timestamps>,
  pub application_id: Option<String>,
  pub details: Option<String>,
  pub state: Option<String>,
  pub sync_id: Option<String>,
  pub flags: Option<u32>,
  pub buttons: Option<Vec<Button>>,
  pub emoji: Option<Emoji>,
  pub party: Option<Party>,
  pub assets: Option<Assets>,
  pub secrets: Option<Secrets>,
  pub metadata: Option<Metadata>,
}

#[skip_serializing_none]
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct TimeoutValue(i64);

#[skip_serializing_none]
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Timestamps {
  #[serde(default)]
  pub start: Option<TimeoutValue>,
  #[serde(default)]
  pub end: Option<TimeoutValue>,
}

#[skip_serializing_none]
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Button {
  pub label: String,
  pub url: String,
}
