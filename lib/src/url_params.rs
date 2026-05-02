use std::collections::HashMap;

pub fn get_url_params(uri: String) -> HashMap<String, String> {
  let mut params = HashMap::new();
  let (_path, query) = match uri.split_once('?') {
    Some(parts) => parts,
    None => return params,
  };

  for param in query.split('&') {
    if let Some((key, value)) = param.split_once('=') {
      params.insert(key.to_string(), value.to_string());
    }
  }

  params
}
