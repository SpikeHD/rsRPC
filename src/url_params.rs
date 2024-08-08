use std::collections::HashMap;

pub fn get_url_params(uri: String) -> HashMap<String, String> {
  let mut params = HashMap::new();
  let uri = uri.split('?').collect::<Vec<&str>>();

  if uri.len() != 2 {
    return params;
  }

  let query = uri[1];

  for param in query.split('&') {
    let param = param.split('=').collect::<Vec<&str>>();
    if param.len() == 2 {
      params.insert(param[0].to_string(), param[1].to_string());
    }
  }

  params
}
