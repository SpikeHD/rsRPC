fn get_socket_path() -> PathBuf {
  let xdg_runtime_dir = env::var("XDG_RUNTIME_DIR").unwrap_or_default();
  let tmpdir = env::var("TMPDIR").unwrap_or_default();
  let tmp = env::var("TMP").unwrap_or_default();
  let temp = env::var("TEMP").unwrap_or_default();
  let tmp_dir = if !xdg_runtime_dir.is_empty() {
      xdg_runtime_dir
  } else if !tmpdir.is_empty() {
      tmpdir
  } else if !tmp.is_empty() {
      tmp
  } else {
      temp
  };

  PathBuf::from(format!("{}/discord-ipc", tmp_dir))
}