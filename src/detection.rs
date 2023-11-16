use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct DetectableActivity {
  #[serde(rename = "bot_public")]
  pub bot_public: Option<bool>,
  #[serde(rename = "bot_require_code_grant")]
  pub bot_require_code_grant: Option<bool>,
  #[serde(rename = "cover_image")]
  pub cover_image: Option<String>,
  pub description: Option<String>,
  #[serde(default)]
  pub developers: Option<Vec<Developer>>,
  #[serde(default)]
  pub executables: Option<Vec<Executable>>,
  pub flags: Option<i64>,
  #[serde(rename = "guild_id")]
  pub guild_id: Option<String>,
  pub hook: bool,
  pub icon: Option<String>,
  pub id: String,
  pub name: String,
  #[serde(default)]
  pub publishers: Vec<Publisher>,
  #[serde(rename = "rpc_origins")]
  #[serde(default)]
  pub rpc_origins: Vec<String>,
  pub splash: Option<String>,
  pub summary: String,
  #[serde(rename = "third_party_skus")]
  #[serde(default)]
  pub third_party_skus: Vec<ThirdPartySku>,
  #[serde(rename = "type")]
  pub type_field: Option<i64>,
  #[serde(rename = "verify_key")]
  pub verify_key: String,
  #[serde(rename = "primary_sku_id")]
  pub primary_sku_id: Option<String>,
  pub slug: Option<String>,
  #[serde(default)]
  pub aliases: Vec<String>,
  pub overlay: Option<bool>,
  #[serde(rename = "overlay_compatibility_hook")]
  pub overlay_compatibility_hook: Option<bool>,
  #[serde(rename = "privacy_policy_url")]
  pub privacy_policy_url: Option<String>,
  #[serde(rename = "terms_of_service_url")]
  pub terms_of_service_url: Option<String>,
  #[serde(rename = "eula_id")]
  pub eula_id: Option<String>,
  #[serde(rename = "deeplink_uri")]
  pub deeplink_uri: Option<String>,
  #[serde(default)]
  pub tags: Vec<String>,
  pub pid: Option<u64>,
  pub timestamp: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Developer {
  pub id: String,
  pub name: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Executable {
  #[serde(rename = "is_launcher")]
  pub is_launcher: bool,
  pub name: String,
  pub os: String,
  pub arguments: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Publisher {
  pub id: String,
  pub name: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ThirdPartySku {
  pub distributor: String,
  pub id: Option<String>,
  pub sku: Option<String>,
}
