use rayon::prelude::*;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::vec;

// Sysinfo traits
use sysinfo::ProcessExt;
use sysinfo::SystemExt;

use crate::logger;

use super::super::DetectableActivity;

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

  pub detectable_list: Vec<DetectableActivity>,
  pub event_sender: mpsc::Sender<ProcessDetectedEvent>,
}

unsafe impl Sync for ProcessServer {}

impl ProcessServer {
  pub fn new(
    detectable: Vec<DetectableActivity>,
    event_sender: mpsc::Sender<ProcessDetectedEvent>,
    thread_count: u16,
  ) -> Self {
    ProcessServer {
      thread_count,
      detected_list: Arc::new(Mutex::new(vec![])),
      detectable_chunks: Arc::new(Mutex::new(vec![])),
      custom_detectables: Arc::new(Mutex::new(vec![])),
      detectable_list: detectable,
      event_sender,
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
        let detected = clone.scan_for_processes();
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

            println!("Sending event");

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

        std::thread::sleep(Duration::from_secs(10));
      }
    });
  }

  pub fn process_list() -> Vec<Exec> {
    let mut processes = Vec::new();
    let sys = sysinfo::System::new_all();

    for proc in sys.processes() {
      processes.push(Exec {
        pid: proc.0.to_string().parse::<u64>().unwrap(),
        path: proc.1.exe().display().to_string(),
      });
    }

    processes
  }

  pub fn scan_for_processes(&self) -> Vec<DetectableActivity> {
    let chunks = self.detectable_chunks.lock().unwrap();
    let processes = ProcessServer::process_list();

    logger::log("Process scan triggered");

    let detected_list: Vec<Vec<DetectableActivity>> = (0..self.thread_count + 1)
      .into_par_iter()  // Parallel iterator from Rayon
      .map(|i| {
        // if this is the last thread, we are supposed to scan the custom detectables
        let mut detectable_chunk: &Vec<DetectableActivity> = &self.custom_detectables.lock().unwrap();

        if i != self.thread_count {
          println!("Scanning chunk {}", i);
          detectable_chunk = &chunks[i as usize];
        }

        detectable_chunk.iter().filter_map(|obj| {
          if let Some(executables) = &obj.executables {
            for executable in executables {
              for process in &processes {
                let process_path = process.path.to_lowercase().replace('\\', "/");
                // Process path (but consistent slashes, so we can compare properly)
                let exec_path = executable.name.replace('\\', "/");
                let found = !process_path.is_empty() && (
                  process_path.contains(&exec_path) || name_no_ext(&process_path).contains(&exec_path)
                );

                if !found {
                  continue;
                }

                let mut new_activity = obj.clone();
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
        }).collect()
      })
      .collect();

    let mut detected_list_flat: Vec<DetectableActivity> =
      detected_list.into_iter().flatten().collect();

    detected_list_flat.shrink_to_fit();

    detected_list_flat
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
