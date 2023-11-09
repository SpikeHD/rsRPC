use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::vec;

use rayon::prelude::*;

use sysinfo::ProcessExt;
use sysinfo::SystemExt;

use crate::logger;

use super::super::DetectableActivity;

#[derive(Clone)]
pub struct Exec {
  pid: u64,
  name: String,
}

#[derive(Clone)]
pub struct ProcessDetectedEvent {
  pub activity: DetectableActivity,
}

#[derive(Clone)]
pub struct ProcessServer {
  detected_list: Arc<Mutex<Vec<DetectableActivity>>>,
  detectable_chunks: Arc<Mutex<Vec<Vec<DetectableActivity>>>>,
  thread_count: u16,

  pub detectable_list: Vec<DetectableActivity>,
  pub event_sender: mpsc::Sender<ProcessDetectedEvent>,

  pub last_pid: Option<u64>,
  pub last_socket_id: Option<String>,
}

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
      detectable_list: detectable,
      event_sender,

      last_pid: None,
      last_socket_id: None,
    }
  }

  pub fn append_detectables(&mut self, detectable: Vec<DetectableActivity>) {
    self.detectable_list.extend(detectable);
  }

  pub fn start(&self) {
    let mut clone = self.clone();
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

        // If the detected list has changed, send a message to the main thread
        for activity in &detected {
          // If the activity is already in the detected list (by ID), skip
          if clone
            .detected_list
            .lock()
            .unwrap()
            .iter()
            .any(|x| x.id == activity.id)
          {
            // Send back the existing activity
            if let Some(found) = clone.detected_list.lock().unwrap().iter().find(|x| x.id == activity.id) {
              logger::log(format!("Found existing activity: {}", found.name));
              clone
                .event_sender
                .send(ProcessDetectedEvent {
                  activity: found.clone(),
                })
                .unwrap();
            }

            continue;
          }

          logger::log("Found new activity...");

          // Find the activity in the detectable list
          let found = activity;

          new_game_detected = true;

          clone.last_pid = found.pid;
          clone.last_socket_id = Some(found.id.clone());

          clone
            .event_sender
            .send(ProcessDetectedEvent {
              activity: found.clone(),
            })
            .unwrap();
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
                publishers: vec![],
                rpc_origins: vec![],
                splash: None,
                summary: "".to_string(),
                third_party_skus: vec![],
                type_field: None,
                verify_key: "".to_string(),
                primary_sku_id: None,
                slug: None,
                aliases: vec![],
                overlay: None,
                overlay_compatibility_hook: None,
                privacy_policy_url: None,
                terms_of_service_url: None,
                eula_id: None,
                deeplink_uri: None,
                tags: vec![],
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
        name: proc.1.name().to_string(),
      });
    }

    processes
  }

  pub fn scan_for_processes(&self) -> Vec<DetectableActivity> {
    let chunks = self.detectable_chunks.lock().unwrap();
    let processes = ProcessServer::process_list();

    logger::log("Process scan triggered");

    let detected_list: Vec<Vec<DetectableActivity>> = (0..8)
      .into_par_iter()  // Parallel iterator from Rayon
      .map(|i| {
        let detectable_chunk = &chunks[i as usize];

        detectable_chunk.iter().filter_map(|obj| {
          if let Some(executables) = &obj.executables {
            for executable in executables {
              for process in &processes {
                let process_name_lowercase = process.name.to_lowercase();
                if executable.name.to_lowercase() == process_name_lowercase
                  || executable.name.to_lowercase() == name_no_ext(process_name_lowercase)
                {
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
          }
          
          None
        }).collect()
      })
      .collect();

    let mut detected_list_flat: Vec<DetectableActivity> = detected_list.into_iter().flatten().collect();
    
    detected_list_flat.shrink_to_fit();

    detected_list_flat
  }
}

pub fn name_no_ext(name: String) -> String {
  if name.contains('.') {
    // Split the name by the dot
    let split: Vec<&str> = name.split('.').collect();

    return split[0].to_string();
  }

  name
}
