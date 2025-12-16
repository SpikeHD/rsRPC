use aho_corasick::{AhoCorasick, PatternID};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::vec;

#[cfg(not(target_os = "linux"))]
use sysinfo::{ProcessRefreshKind, RefreshKind, System, UpdateKind};

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
  pub activity: DetectableActivity,
}

#[derive(Clone)]
pub struct ProcessServer {
  detected_list: Arc<Mutex<Vec<DetectableActivity>>>,
  custom_detectables: Arc<Mutex<Vec<DetectableActivity>>>,
  scanning: Arc<AtomicBool>,

  detectable_indexes: Arc<Mutex<Vec<[usize; 2]>>>,
  detectable_ac: Arc<Mutex<AhoCorasick>>,

  custom_detectable_indexes: Arc<Mutex<Vec<[usize; 2]>>>,
  custom_detectable_ac: Arc<Mutex<Option<AhoCorasick>>>,

  pub detectable_list: Vec<DetectableActivity>,
  pub event_sender: mpsc::Sender<ProcessDetectedEvent>,

  event_listeners: Arc<Mutex<ProcessEventListeners>>,

  #[cfg(not(target_os = "linux"))]
  sysinfo: Arc<Mutex<System>>,
}

unsafe impl Sync for ProcessServer {}

impl ProcessServer {
  pub fn new(
    detectable: Vec<DetectableActivity>,
    event_sender: mpsc::Sender<ProcessDetectedEvent>,
    event_listeners: ProcessEventListeners,
  ) -> Self {
    log!("[Process Scanner] Building Aho-Corasick patterns for main detectable activities...");
    let (ac, idx) = build_ac_patterns(&detectable);
    log!("[Process Scanner] Done!");

    ProcessServer {
      scanning: Arc::new(AtomicBool::new(false)),
      detected_list: Arc::new(Mutex::new(vec![])),
      custom_detectables: Arc::new(Mutex::new(vec![])),
      detectable_list: detectable,
      event_sender,

      // Aho-Corasick matching with detectables mapping
      detectable_indexes: Arc::new(Mutex::new(idx)),
      detectable_ac: Arc::new(Mutex::new(ac)),
      custom_detectable_indexes: Arc::new(Mutex::new(vec![])),
      custom_detectable_ac: Arc::new(Mutex::new(None)),

      // Event listeners
      event_listeners: Arc::new(Mutex::new(event_listeners)),

      // sysinfo System
      #[cfg(not(target_os = "linux"))]
      sysinfo: Arc::new(Mutex::new(System::new_with_specifics(
        RefreshKind::nothing().with_processes(
          ProcessRefreshKind::nothing()
            .with_exe(UpdateKind::Always)
            .with_cmd(UpdateKind::Always),
        ),
      ))),
    }
  }

  fn update_custom_detectables(&self) {
    log!("[Process Scanner] Updating Aho-Corasick patterns for custom detectable activities...");
    let (ac, idx) = build_ac_patterns(&self.custom_detectables.lock().unwrap());
    if !idx.is_empty() {
      *self.custom_detectable_ac.lock().unwrap() = Some(ac);
    } else {
      *self.custom_detectable_ac.lock().unwrap() = None;
    }
    *self.custom_detectable_indexes.lock().unwrap() = idx;
    log!("[Process Scanner] Done!");
  }

  pub fn append_detectables(&mut self, mut detectable: Vec<DetectableActivity>) {
    // Append to detectable chunks, since that's what is actually scanned
    self
      .custom_detectables
      .lock()
      .unwrap()
      .append(&mut detectable);
    self.update_custom_detectables();
  }

  pub fn remove_detectable_by_name(&mut self, name: String) {
    self
      .custom_detectables
      .lock()
      .unwrap()
      .retain(|x| x.name != name);
    self.update_custom_detectables();
  }

  pub fn start(&self) {
    let wait_time = Duration::from_secs(10);
    let clone = self.clone();

    self.update_custom_detectables();

    std::thread::spawn(move || {
      // Run the process scan repeatedly (every 3 seconds)
      loop {
        let detected = match clone.scan_for_processes() {
          Ok(detected) => detected,
          Err(err) => {
            log!("[Process Scanner] Error while scanning processes: {}", err);
            std::thread::sleep(wait_time);
            continue;
          }
        };
        let mut new_game_detected = false;

        // If the detected list has changed, send only the first element
        if !detected.is_empty() {
          let detected_list = clone.detected_list.lock().unwrap();

          // If the detected list is empty, send the first element
          if detected_list.is_empty() {
            new_game_detected = true;
            clone
              .event_sender
              .send(ProcessDetectedEvent {
                activity: detected[0].clone(),
              })
              .unwrap();
          } else {
            // If the detected list is not empty, check if the first element is different
            if detected[0].id != detected_list[0].id {
              new_game_detected = true;
            }

            clone
              .event_sender
              .send(ProcessDetectedEvent {
                activity: detected[0].clone(),
              })
              .unwrap();
          }
        }

        // If there are no detected processes, send an empty message
        if detected.is_empty() {
          clone
            .event_sender
            .send(ProcessDetectedEvent {
              activity: DetectableActivity {
                bot_public: None,
                bot_require_code_grant: None,
                cover_image: None,
                description: None,
                developers: None,
                executables: None,
                flags: None,
                guild_id: None,
                hook: false,
                icon: None,
                id: "null".to_string(),
                name: "".to_string(),
                publishers: None,
                rpc_origins: None,
                splash: None,
                third_party_skus: None,
                type_field: None,
                verify_key: None,
                primary_sku_id: None,
                slug: None,
                aliases: None,
                overlay: None,
                overlay_compatibility_hook: None,
                privacy_policy_url: None,
                terms_of_service_url: None,
                eula_id: None,
                deeplink_uri: None,
                tags: None,
                pid: None,
                timestamp: None,
              },
            })
            .unwrap();
        }

        if new_game_detected {
          // Set the detected list to the new list
          *clone.detected_list.lock().unwrap() = detected;
        }

        std::thread::sleep(wait_time);
      }
    });
  }

  #[cfg(not(target_os = "linux"))]
  pub fn process_list(&self) -> Result<Vec<Exec>, Box<dyn std::error::Error>> {
    use std::path::Path;

    let mut processes = Vec::new();
    let mut sys = self.sysinfo.lock().unwrap();
    sys.refresh_processes_specifics(
      sysinfo::ProcessesToUpdate::All,
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
        return entry.file_name().to_str().unwrap().parse::<u64>().is_ok();
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

  pub fn scan_for_processes(&self) -> Result<Vec<DetectableActivity>, Box<dyn std::error::Error>> {
    #[cfg(not(target_os = "linux"))]
    let processes = self.process_list()?;
    #[cfg(target_os = "linux")]
    let processes = ProcessServer::process_list()?;

    log!("[Process Scanner] Process scan triggered");

    if self.scanning.load(std::sync::atomic::Ordering::Relaxed) {
      log!("[Process Scanner] Scanning already in progress");
      return Err("Scanning already in progress".into());
    }

    let process_scan_state = Mutex::new(ProcessScanState::default());

    let ac = self.detectable_ac.lock().unwrap();
    let custom_ac = self.custom_detectable_ac.lock().unwrap();

    let mut detected_list: Vec<DetectableActivity> = processes
      .iter()
      .filter_map(|process| {
        // Process path (but consistent slashes, so we can compare properly)
        let process_path = process.path.to_lowercase().replace('\\', "/");

        if process_path.contains("obs64") || process_path.contains("streamlabs") {
          process_scan_state.lock().unwrap().obs_open = true;
        }

        // Aho-Corasick matching
        let reversed_path: String = process_path.chars().rev().collect();
        let (obj, exe_index) = if let Some(mat) = ac.find(&reversed_path) {
          let pattern_id: PatternID = mat.pattern();
          let exe_index = self.detectable_indexes.lock().unwrap()[pattern_id.as_usize()];
          (&self.detectable_list[exe_index[0]], exe_index[1])
        } else if custom_ac.is_some() {
          let custom_ac = custom_ac.as_ref().unwrap();
          if let Some(mat) = custom_ac.find(&reversed_path) {
            let pattern_id: PatternID = mat.pattern();
            let exe_index = self.custom_detectable_indexes.lock().unwrap()[pattern_id.as_usize()];
            (
              &self.custom_detectables.lock().unwrap()[exe_index[0]],
              exe_index[1],
            )
          } else {
            return None;
          }
        } else {
          return None;
        };

        // Argument checks
        let mut new_activity = obj.clone();
        let executable = &obj.executables.as_ref().unwrap()[exe_index];

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

        new_activity.pid = Some(process.pid);
        new_activity.timestamp = Some(format!(
          "{:?}",
          std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
        ));
        Some(new_activity)
      })
      .collect();

    if let Some(callback) = self
      .event_listeners
      .lock()
      .unwrap()
      .on_process_scan_complete
      .as_ref()
    {
      callback.lock().unwrap()(process_scan_state.lock().unwrap().clone());
    }

    detected_list.shrink_to_fit();

    log!("[Process Scanner] Process scan complete");

    Ok(detected_list)
  }
}

fn build_ac_patterns(detectables: &[DetectableActivity]) -> (AhoCorasick, Vec<[usize; 2]>) {
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

        exe_patterns.push(exec_name.chars().rev().collect::<String>());
        exe_indexes.push([activity_index, exe_index]);
      }
    }
  }

  (AhoCorasick::new(exe_patterns).unwrap(), exe_indexes)
}

// pub fn name_no_ext(name: &String) -> String {
//   if name.contains('.') {
//     // Split the name by the dot
//     let split: Vec<&str> = name.split('.').collect();

//     return split[0].to_string();
//   }

//   name.to_owned()
// }
