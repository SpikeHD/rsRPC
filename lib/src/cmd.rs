use serde::{Deserialize, Serialize};
use serde_json::Value;
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
  pub nonce: Value,
}

impl ActivityCmd {
  pub fn empty() -> Self {
    Self {
      application_id: None,
      cmd: "".to_string(),
      args: None,
      data: None,
      evt: None,
      nonce: Value::String("".to_string()),
    }
  }

  pub fn fix(&mut self) {
    self.fix_timestamps();
    self.fix_buttons();
    self.fix_flags();
  }

  pub fn fix_timestamps(&mut self) {
    if let Some(timestamps) = self
      .args
      .as_mut()
      .and_then(|args| args.activity.as_mut())
      .and_then(|activity| activity.timestamps.as_mut())
    {
      let cur = chrono::Utc::now().timestamp() + (100 * 365 * 24 * 3600);

      // convert starting timestamp
      if let Some(start) = timestamps.start.as_mut() {
        // convert timestamp if in seconds
        if start.0 > cur {
          *start = TimeoutValue(start.0);
        } else {
          *start = TimeoutValue(start.0 * 1000);
        }
      }

      // convert ending timestamp
      if let Some(end) = timestamps.end.as_mut() {
        // convert timestamp if in seconds
        if end.0 > cur {
          *end = TimeoutValue(end.0);
        } else {
          *end = TimeoutValue(end.0 * 1000);
        }
      }
    }
  }

  pub fn fix_buttons(&mut self) {
    // If `buttons` are an array of objects, we need to map the labels to `buttons` (as a string array) and the urls to `metadata.button_urls` (as an array of strings)
    if let Some(activity) = self.args.as_mut().and_then(|args| args.activity.as_mut()) {
      if let Some(buttons) = activity.buttons.as_mut() {
        let mut button_urls: Vec<String> = vec![];
        let mut button_labels: Vec<Value> = vec![];

        for b in buttons {
          if let Some(label) = b.get("label") {
            // Unless the provider of the actvity REALLY screwed up, we can safely assume this is a string
            button_labels.push(label.clone());
          }
          if let Some(url) = b.get("url") {
            button_urls.push(url.as_str().unwrap_or("").to_string());
          }
        }

        activity.metadata = Some(Metadata {
          button_urls: Some(button_urls),
          ..activity.metadata.clone().unwrap_or_default()
        });

        activity.buttons = Some(button_labels);
      }
    }
  }

  pub fn fix_flags(&mut self) {
    if let Some(activity) = self.args.as_mut().and_then(|args| args.activity.as_mut()) {
      if activity.instance.unwrap_or(false) && activity.flags.is_none() {
        activity.flags = Some(1);
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
  pub buttons: Option<Vec<Value>>,
  #[serde(default)]
  pub r#type: u32,
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
  pub instance: Option<bool>,
  pub flags: Option<u32>,
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
