pub fn connection_resp() -> &'static str {
  r#"
      {
        "cmd": "DISPATCH",
        "evt": "READY",
        "data": {
          "v": 1,
          "user": {
            "id": "1045800378228281345",
            "username": "arRPC",
            "discriminator": "0000",
            "avatar": "cfefa4d9839fb4bdf030f91c2a13e95c",
            "flags": 0,
            "premium_type": 0
          },
          "config": {
            "api_endpoint": "//discord.com/api",
            "cdn_host": "cdn.discordapp.com",
            "environment": "production"
          }
        }
      }
  "#
}
