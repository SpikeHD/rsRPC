use aho_corasick::AhoCorasick;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

#[cfg(not(target_os = "linux"))]
use sysinfo::System;

use crate::detection::null_activity;
use crate::log;
use crate::ProcessCallback;

use super::super::DetectableActivity;

#[derive(Default, Clone)]
pub struct ProcessScanState {
  pub obs_open: bool,
}

#[derive(Default)]
pub struct ProcessEventListeners {
  pub on_process_scan_complete: Option<Arc<Mutex<ProcessCallback>>>,
}

#[derive(Clone)]
pub struct Exec {
  pid: u64,
  path: String,
  arguments: Option<String>,
}

#[derive(Clone)]
pub struct ProcessDetectedEvent {
  pub activity: Arc<DetectableActivity>,
}

type Detectables = Vec<Arc<DetectableActivity>>;
type AcPatterns = (AhoCorasick, Vec<[usize; 2]>);

#[derive(Clone)]
pub struct DetectState {
  pub detectables: Detectables,
  pub ac: Option<AhoCorasick>,
  pub indexes: Vec<[usize; 2]>,
}

#[derive(Clone)]
pub struct ProcessServer {
  detected_list: Arc<Mutex<Detectables>>,
  scanning: Arc<AtomicBool>,

  detectable_state: Arc<Mutex<DetectState>>,
  custom_state: Arc<Mutex<DetectState>>,

  pub event_sender: mpsc::Sender<ProcessDetectedEvent>,

  event_listeners: Arc<Mutex<ProcessEventListeners>>,

  #[cfg(not(target_os = "linux"))]
  sysinfo: Arc<Mutex<System>>,
}

unsafe impl Sync for ProcessServer {}

impl ProcessServer {
  pub fn new(
    detectable: Detectables,
    event_sender: mpsc::Sender<ProcessDetectedEvent>,
    event_listeners: ProcessEventListeners,
  ) -> Self {
    log!("[Process Scanner] Building Aho-Corasick patterns for main detectable activities...");
    let state = build_detect_state(detectable);
    log!("[Process Scanner] Done!");

    ProcessServer {
      scanning: Arc::new(AtomicBool::new(false)),
      detected_list: Arc::new(Mutex::new(vec![])),
      event_sender,

      // Aho-Corasick matching with detectables mapping
      detectable_state: Arc::new(Mutex::new(state)),
      custom_state: Arc::new(Mutex::new(build_detect_state(vec![]))),

      // Event listeners
      event_listeners: Arc::new(Mutex::new(event_listeners)),

      // sysinfo System
      #[cfg(not(target_os = "linux"))]
      sysinfo: Arc::new(Mutex::new(System::new())),
    }
  }

  fn update_custom_detectables(&self) {
    log!("[Process Scanner] Updating Aho-Corasick patterns for custom detectable activities...");
    let mut state = self.custom_state.lock().unwrap();
    rebuild_custom_detectables(&mut state);
    log!("[Process Scanner] Done!");
  }

  pub fn append_detectables(&mut self, detectable: Vec<DetectableActivity>) {
    // Append to detectable chunks, since that's what is actually scanned
    let mut state = self.custom_state.lock().unwrap();
    state
      .detectables
      .extend(detectable.into_iter().map(Arc::new));
    rebuild_custom_detectables(&mut state);
  }

  pub fn remove_detectable_by_name(&mut self, name: String) {
    let mut state = self.custom_state.lock().unwrap();
    state.detectables.retain(|x| x.name != name);
    rebuild_custom_detectables(&mut state);
  }

  pub fn start(&self) {
    let wait_time = Duration::from_secs(10);
    let clone = self.clone();

    self.update_custom_detectables();

    // Reused when a scan finds nothing
    let no_activity = Arc::new(null_activity());

    std::thread::spawn(move || {
      // Run the process scan repeatedly (every wait_time)
      loop {
        let detected = match clone.scan_for_processes() {
          Ok(detected) => detected,
          Err(err) => {
            log!("[Process Scanner] Error while scanning processes: {}", err);
            std::thread::sleep(wait_time);
            continue;
          }
        };

        let activity: Arc<DetectableActivity> = match detected.is_empty() {
          true => no_activity.clone(),
          false => detected[0].clone(),
        };

        // Stop the loop if the receiver went away
        let mut previous = clone.detected_list.lock().unwrap();

        // A new game is only "detected" when the id differs
        let new_game_detected =
          !detected.is_empty() && (previous.is_empty() || detected[0].id != previous[0].id);

        if let Err(err) = clone.event_sender.send(ProcessDetectedEvent { activity }) {
          log!("[Process Scanner] Event receiver is gone: {:?}", err);
          break;
        }

        if new_game_detected {
          // Set the detected list to the new list
          *previous = detected;
        }

        std::thread::sleep(wait_time);
      }
    });
  }

  #[cfg(not(target_os = "linux"))]
  pub fn process_list(&self) -> Result<Vec<Exec>, Box<dyn std::error::Error>> {
    use std::path::Path;
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, UpdateKind};

    let mut processes = Vec::new();
    let mut sys = self.sysinfo.lock().unwrap();
    sys.refresh_processes_specifics(
      ProcessesToUpdate::All,
      true,
      ProcessRefreshKind::nothing()
        .with_exe(UpdateKind::OnlyIfNotSet)
        .with_cmd(UpdateKind::OnlyIfNotSet),
    );

    for proc in sys.processes() {
      let mut cmd = proc.1.cmd().iter();
      processes.push(Exec {
        pid: proc.0.to_string().parse::<u64>()?,
        path: proc.1.exe().unwrap_or(Path::new("")).display().to_string(),
        arguments: cmd.next().map(|_| {
          cmd
            .map(|x| x.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ")
        }),
      });
    }

    Ok(processes)
  }

  #[cfg(target_os = "linux")]
  pub fn process_list() -> Result<Vec<Exec>, Box<dyn std::error::Error>> {
    use std::fs;

    let proc_list = fs::read_dir("/proc")?.filter(|e| {
      if let Ok(entry) = e {
        // Only if we can parse this as a number
        return entry
          .file_name()
          .to_str()
          .unwrap_or_default()
          .parse::<u64>()
          .is_ok();
      }

      false
    });
    let mut processes = Vec::new();

    for entry in proc_list {
      let entry = entry?;
      let path = entry.path();

      if let Ok(cmdline) = fs::read_to_string(path.join("cmdline")) {
        if !cmdline.is_empty() {
          let mut cmd_iter = cmdline.split('\0');
          let (cmd_path, cmd_args) = (
            cmd_iter.next().unwrap_or("").to_string(),
            cmd_iter.collect::<Vec<_>>().join(" "),
          );
          processes.push(Exec {
            pid: path
              .file_name()
              .ok_or("Invalid path")?
              .to_str()
              .ok_or("Invalid path")?
              .parse::<u64>()?,
            path: cmd_path,
            arguments: if cmd_args.is_empty() {
              None
            } else {
              Some(cmd_args)
            },
          });
        }
      }
    }

    Ok(processes)
  }

  pub fn scan_for_processes(
    &self,
  ) -> Result<Vec<Arc<DetectableActivity>>, Box<dyn std::error::Error>> {
    #[cfg(not(target_os = "linux"))]
    let processes = self.process_list()?;
    #[cfg(target_os = "linux")]
    let processes = ProcessServer::process_list()?;

    log!("[Process Scanner] Process scan triggered");

    if self.scanning.load(std::sync::atomic::Ordering::Relaxed) {
      log!("[Process Scanner] Scanning already in progress");
      return Err("Scanning already in progress".into());
    }

    let mut obs_open = false;

    let main_state = self.detectable_state.lock().unwrap();
    let custom_state = self.custom_state.lock().unwrap();

    let mut reversed_path = String::with_capacity(256);

    let mut detected_list: Vec<Arc<DetectableActivity>> = processes
      .iter()
      .filter_map(|process| {
        // Process path (but consistent slashes, so we can compare properly)
        let mut process_path = process.path.to_ascii_lowercase();

        if process_path.contains('\\') {
          process_path = process_path.replace('\\', "/");
        }

        if !process_path.starts_with('/') {
          process_path.insert(0, '/');
        }

        if !obs_open && (process_path.contains("obs64") || process_path.contains("streamlabs")) {
          obs_open = true;
        }

        // Aho-Corasick matching
        reversed_path.clear();
        reversed_path.extend(process_path.chars().rev());

        let (obj, exe_index) = match first_match(&main_state, &reversed_path)
          .or_else(|| first_match(&custom_state, &reversed_path))
        {
          None => return None,
          Some((activity, exe_index)) => (activity, exe_index),
        };

        // Argument checks
        let executable = match obj.executables.as_ref().and_then(|exe| exe.get(exe_index)) {
          None => return None,
          Some(executable) => executable,
        };

        if let Some(exec_args) = &executable.arguments {
          // Only require argument checks if executable starts with '>'
          // like Minecraft: { arguments: "net.minecraft.client.main.Main", is_launcher: false, name: ">java", … }
          // Other games might provide arguments but not necessary be checked
          // like Left 4 Dead 2: { arguments: "-game left4dead2", is_launcher: false, name: "left 4 dead 2/left4dead2.exe", … }
          if executable.name.starts_with(">")
            && !process
              .arguments
              .as_ref()
              .is_some_and(|args| args.contains(exec_args))
          {
            return None;
          }
        }

        let mut new_activity = (**obj).clone();
        new_activity.pid = Some(process.pid);
        new_activity.timestamp = Some(format!(
          "{:?}",
          std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
        ));
        Some(Arc::new(new_activity))
      })
      .collect();

    // A user callback may panic, ignore the poison
    if let Some(callback) = self
      .event_listeners
      .lock()
      .unwrap_or_else(|err| err.into_inner())
      .on_process_scan_complete
      .as_ref()
    {
      let mut callback = callback.lock().unwrap_or_else(|err| err.into_inner());

      callback(ProcessScanState { obs_open });
    }

    detected_list.shrink_to_fit();

    log!("[Process Scanner] Process scan complete");

    Ok(detected_list)
  }
}

fn build_detect_state(detectables: Detectables) -> DetectState {
  match build_ac_patterns(&detectables) {
    Err(err) => {
      log!(
        "[Process Scanner] Patterns are unusable, nothing will match: {:?}",
        err
      );
      DetectState {
        detectables,
        ac: None,
        indexes: vec![],
      }
    }
    Ok((ac, indexes)) => DetectState {
      detectables,
      ac: Some(ac),
      indexes,
    },
  }
}

fn rebuild_custom_detectables(state: &mut DetectState) {
  match build_ac_patterns(&state.detectables) {
    Err(err) => log!(
      "[Process Scanner] Custom patterns are unusable, keeping the old automaton: {:?}",
      err
    ),
    Ok((ac, indexes)) => {
      // No patterns means there's nothing worth matching against
      state.ac = if indexes.is_empty() { None } else { Some(ac) };
      state.indexes = indexes;
    }
  }
}

fn first_match(
  state: &DetectState,
  reversed_path: impl AsRef<str>,
) -> Option<(&Arc<DetectableActivity>, usize)> {
  state
    .ac
    .as_ref()
    .and_then(|ac| ac.find(reversed_path.as_ref()))
    .and_then(|mat| state.indexes.get(mat.pattern().as_usize()))
    .and_then(|exe_map| {
      state
        .detectables
        .get(exe_map[0])
        .map(|activity| (activity, exe_map[1]))
    })
}

fn build_ac_patterns(detectables: &Detectables) -> Result<AcPatterns, Box<dyn std::error::Error>> {
  let mut exe_patterns: Vec<String> = Vec::new();
  let mut exe_indexes: Vec<[usize; 2]> = Vec::new();

  for (activity_index, activity) in detectables.iter().enumerate() {
    if let Some(executables) = &activity.executables {
      for (exe_index, executable) in executables.iter().enumerate() {
        if executable.is_launcher {
          continue;
        }

        // Make paths consistent, and fix some additional checks
        let mut exec_name = executable.name.replace('\\', "/").to_lowercase();

        // Checks adapted from arrpc, remain the '>' in DetectableActivity for later argument checks
        if exec_name.starts_with(">") {
          exec_name.replace_range(0..1, "/");
        } else if !exec_name.starts_with("/") {
          exec_name.insert(0, '/');
        }

        if exec_name.is_empty() {
          continue;
        }

        exe_patterns.push(exec_name.chars().rev().collect::<String>());
        exe_indexes.push([activity_index, exe_index]);
      }
    }
  }

  match AhoCorasick::new(exe_patterns) {
    Err(err) => Err(err.to_string().into()),
    Ok(ac) => Ok((ac, exe_indexes)),
  }
}
