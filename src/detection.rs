use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DetectableActivity {
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(rename = "bot_public")]
  pub bot_public: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(rename = "bot_require_code_grant")]
  pub bot_require_code_grant: Option<bool>,
  #[serde(rename = "cover_image")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub cover_image: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub description: Option<String>,
  #[serde(default)]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub developers: Option<Vec<Developer>>,
  #[serde(default)]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub executables: Option<Vec<Executable>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub flags: Option<i64>,
  #[serde(rename = "guild_id")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub guild_id: Option<String>,
  pub hook: bool,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub icon: Option<String>,
  pub id: String,
  pub name: String,
  #[serde(default)]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub publishers: Option<Vec<Publisher>>,
  #[serde(rename = "rpc_origins")]
  #[serde(default)]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub rpc_origins: Option<Vec<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub splash: Option<String>,
  #[serde(rename = "third_party_skus")]
  #[serde(default)]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub third_party_skus: Option<Vec<ThirdPartySku>>,
  #[serde(rename = "type")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub type_field: Option<i64>,
  #[serde(rename = "verify_key")]
  pub verify_key: Option<String>,
  #[serde(rename = "primary_sku_id")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub primary_sku_id: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub slug: Option<String>,
  #[serde(default)]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub aliases: Option<Vec<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub overlay: Option<bool>,
  #[serde(rename = "overlay_compatibility_hook")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub overlay_compatibility_hook: Option<bool>,
  #[serde(rename = "privacy_policy_url")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub privacy_policy_url: Option<String>,
  #[serde(rename = "terms_of_service_url")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub terms_of_service_url: Option<String>,
  #[serde(rename = "eula_id")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub eula_id: Option<String>,
  #[serde(rename = "deeplink_uri")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub deeplink_uri: Option<String>,
  #[serde(default)]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub tags: Option<Vec<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub pid: Option<u64>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub timestamp: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Developer {
  pub id: String,
  pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Executable {
  #[serde(rename = "is_launcher")]
  pub is_launcher: bool,
  pub name: String,
  pub os: String,
  pub arguments: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Publisher {
  pub id: String,
  pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ThirdPartySku {
  pub distributor: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub id: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub sku: Option<String>,
}
