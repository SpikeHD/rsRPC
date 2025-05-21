use rayon::prelude::*;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::vec;
use sysinfo::UpdateKind;
use sysinfo::{ProcessRefreshKind, RefreshKind, System};

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
}

#[derive(Clone)]
pub struct ProcessDetectedEvent {
  pub activity: DetectableActivity,
}

#[derive(Clone)]
pub struct ProcessServer {
  detected_list: Arc<Mutex<Vec<DetectableActivity>>>,
  detectable_chunks: Arc<Mutex<Vec<Vec<DetectableActivity>>>>,
  custom_detectables: Arc<Mutex<Vec<DetectableActivity>>>,
  thread_count: u16,
  scanning: Arc<AtomicBool>,

  pub detectable_list: Vec<DetectableActivity>,
  pub event_sender: mpsc::Sender<ProcessDetectedEvent>,

  event_listeners: Arc<Mutex<ProcessEventListeners>>,
}

unsafe impl Sync for ProcessServer {}

impl ProcessServer {
  pub fn new(
    detectable: Vec<DetectableActivity>,
    event_sender: mpsc::Sender<ProcessDetectedEvent>,
    thread_count: u16,
    event_listeners: ProcessEventListeners,
  ) -> Self {
    ProcessServer {
      scanning: Arc::new(AtomicBool::new(false)),
      thread_count,
      detected_list: Arc::new(Mutex::new(vec![])),
      detectable_chunks: Arc::new(Mutex::new(vec![])),
      custom_detectables: Arc::new(Mutex::new(vec![])),
      detectable_list: detectable,
      event_sender,

      // Event listeners
      event_listeners: Arc::new(Mutex::new(event_listeners)),
    }
  }

  pub fn append_detectables(&mut self, mut detectable: Vec<DetectableActivity>) {
    // Append to detectable chunks, since that's what is actually scanned
    self
      .custom_detectables
      .lock()
      .unwrap()
      .append(&mut detectable);
  }

  pub fn remove_detectable_by_name(&mut self, name: String) {
    self
      .custom_detectables
      .lock()
      .unwrap()
      .retain(|x| x.name != name);
  }

  pub fn start(&self) {
    let wait_time = Duration::from_secs(10);
    let clone = self.clone();
    // Evenly split the detectable list into chunks
    let mut chunks: Vec<Vec<DetectableActivity>> = vec![];

    for _ in 0..self.thread_count {
      chunks.push(vec![]);
    }

    let mut i = 0;

    for obj in &self.detectable_list {
      chunks[i].push(obj.clone());

      i += 1;

      if i >= self.thread_count.into() {
        i = 0;
      }
    }

    *clone.detectable_chunks.lock().unwrap() = chunks;

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
  pub fn process_list() -> Result<Vec<Exec>, Box<dyn std::error::Error>> {
    let mut processes = Vec::new();
    let sys = System::new_with_specifics(
      RefreshKind::nothing()
        .with_processes(ProcessRefreshKind::nothing().with_exe(UpdateKind::Always)),
    );

    for proc in sys.processes() {
      processes.push(Exec {
        pid: proc.0.to_string().parse::<u64>()?,
        path: proc.1.exe().unwrap_or(Path::new("")).display().to_string(),
      });
    }

    Ok(processes)
  }

  #[cfg(target_os = "linux")]
  pub fn process_list() -> Result<Vec<Exec>, Box<dyn std::error::Error>> {
    use std::fs;

    let mut proc_list = fs::read_dir("/proc")?.filter(|e| {
      if let Ok(entry) = e {
        // Only if we can parse this as a number
        return entry.file_name().to_str().unwrap().parse::<u64>().is_ok();
      }
  
      false
    });
    let mut processes = Vec::new();
  
    while let Some(entry) = proc_list.next() {
      let entry = entry?;
      let path = entry.path();
  
      if let Ok(cmdline) = fs::read_to_string(path.join("cmdline")) {
        if !cmdline.is_empty() {
          processes.push(Exec {
            pid: path.file_name().ok_or("Invalid path")?.to_str().ok_or("Invalid path")?.parse::<u64>()?,
            path: cmdline.split('\0').next().unwrap_or("").to_string(),
          });
        }
      }
    }
  
    Ok(processes)
  }

  pub fn scan_for_processes(&self) -> Result<Vec<DetectableActivity>, Box<dyn std::error::Error>> {
    let chunks = self.detectable_chunks.lock().unwrap();
    let processes = ProcessServer::process_list()?;

    log!("[Process Scanner] Process scan triggered");

    if self.scanning.load(std::sync::atomic::Ordering::Relaxed) {
      log!("[Process Scanner] Scanning already in progress");
      return Err("Scanning already in progress".into());
    }

    let process_scan_state = Mutex::new(ProcessScanState::default());

    let mut detected_list: Vec<DetectableActivity> = (0..self.thread_count + 1)
      .into_par_iter()
      .flat_map(|i| {
        // if this is the last thread, we are supposed to scan the custom detectables
        let detectable_chunk: &Vec<DetectableActivity> = if self.thread_count == i {
          &self.custom_detectables.lock().unwrap()
        } else {
          &chunks[i as usize]
        };

        detectable_chunk
          .iter()
          .filter_map(|obj| {
            let mut new_activity = obj.clone();

            if let Some(executables) = &obj.executables {

              for executable in executables {
                if executable.is_launcher {
                  continue
                }

                let exec_path = executable.name.replace('\\', "/").to_lowercase();

                for process in &processes {
                  // Process path (but consistent slashes, so we can compare properly)
                  let process_path = process.path.to_lowercase().replace('\\', "/");

                  if process_path.contains("obs64") || process_path.contains("streamlabs") {
                    process_scan_state.lock().unwrap().obs_open = true;
                  }

                  // Checks adapted from [arrpc](https://github.com/OpenAsar/arrpc/blob/2234e9c9111f4c42ebcc3aa6a2215bfd979eef77/src/process/index.js#L54)
                  let found = if executable.name.starts_with('>') {
                    exec_path[1..] == process_path
                  } else {
                    process_path.contains(&exec_path)
                  };

                  if !found {
                    continue;
                  }

                  new_activity.pid = Some(process.pid);
                  new_activity.timestamp = Some(format!(
                    "{:?}",
                    std::time::SystemTime::now()
                      .duration_since(std::time::UNIX_EPOCH)
                      .unwrap()
                      .as_millis()
                  ));
                  return Some(new_activity);
                }
              }
            }
            None
          })
          .collect::<Vec<DetectableActivity>>()
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

pub fn name_no_ext(name: &String) -> String {
  if name.contains('.') {
    // Split the name by the dot
    let split: Vec<&str> = name.split('.').collect();

    return split[0].to_string();
  }

  name.to_owned()
}
