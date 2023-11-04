use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::vec;

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

pub struct ProcessServer {
  detected_list: Arc<Mutex<Vec<DetectableActivity>>>,
  detectable_chunks: Arc<Mutex<Vec<Vec<DetectableActivity>>>>,
  thread_count: u16,

  // ms to wait in between each process scan
  pub scan_wait_ms: u64,
  pub detectable_list: Vec<DetectableActivity>,
  pub event_sender: mpsc::Sender<ProcessDetectedEvent>,
}

impl ProcessServer {
  pub fn new(
    detectable: Vec<DetectableActivity>,
    event_sender: mpsc::Sender<ProcessDetectedEvent>,
    thread_count: u16,
  ) -> Self {
    ProcessServer {
      thread_count: thread_count,
      detected_list: Arc::new(Mutex::new(vec![])),
      detectable_chunks: Arc::new(Mutex::new(vec![])),
      scan_wait_ms: 1,
      detectable_list: detectable,
      event_sender,
    }
  }

  pub fn append_detectables(&mut self, detectable: Vec<DetectableActivity>) {
    self.detectable_list.extend(detectable);
  }

  pub fn start(self) {
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

    *self.detectable_chunks.lock().unwrap() = chunks;

    std::thread::spawn(move || {
      // Run the process scan repeatedly (every 3 seconds)
      loop {
        let detected = self.scan_for_processes();
        let mut new_game_detected = false;

        // If the detected list has changed, send a message to the main thread
        for activity in &detected {
          // If the activity is already in the detected list (by ID), skip
          if self
            .detected_list
            .lock()
            .unwrap()
            .iter()
            .any(|x| x.id == activity.id)
          {
            // Send back the existing activity
            let found = self
              .detected_list
              .lock()
              .unwrap()
              .iter()
              .find(|x| x.id == activity.id)
              .unwrap()
              .clone();

            logger::log(format!("Found existing activity: {}", found.name));

            self
              .event_sender
              .send(ProcessDetectedEvent {
                activity: found.clone(),
              })
              .unwrap();

            continue;
          }

          logger::log("Found new activity...");

          // Find the activity in the detectable list
          let found = activity;

          new_game_detected = true;

          self
            .event_sender
            .send(ProcessDetectedEvent {
              activity: found.clone(),
            })
            .unwrap();
        }

        // If there are no detected processes, send an empty message
        if detected.is_empty() {
          self
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
          *self.detected_list.lock().unwrap() = detected;
        }

        std::thread::sleep(Duration::from_secs(5));
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
    let mut detected_list = vec![];
    let chunks = self.detectable_chunks.lock().unwrap().clone();
    let scan_wait_ms = self.scan_wait_ms;

    logger::log("Process scan triggered");

    // Create a pool of threads, and split the detectable list into chunks
    let mut thread_handles = vec![];

    for i in 0..self.thread_count {
      let detectable_list = chunks[i as usize].clone();
      let mut thread_detected_list = vec![];
      let processes = ProcessServer::process_list();

      let thread = std::thread::spawn(move || {
        for obj in detectable_list {
          // if executables is null, just skip
          if obj.executables.is_none() {
            continue;
          }
    
          // It's fine if this is a little slow so as to not crank the CPU
          std::thread::sleep(std::time::Duration::from_millis(scan_wait_ms));
    
          for process in &processes {
            // detectable['executables'] is an array of objects with keys is_launcher, name, and os
            for executable in obj.executables.as_ref().unwrap() {
              // If this game is not in the list of already detected games, and the executable name matches, add
              if executable.name.to_lowercase() == *process.name.to_lowercase()
                || executable.name.to_lowercase() == name_no_ext(process.name.to_lowercase())
              {
                // Push the whole game
                let mut new_activity = obj.clone();
                new_activity.pid = Some(process.pid);
    
                // Set timestamp to JS timestamp
                new_activity.timestamp = Some(format!(
                  "{:?}",
                  std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis()
                ));
    
                thread_detected_list.push(new_activity);
              }
            }
          }
        }

        thread_detected_list
      });

      thread_handles.push(thread);
    }

    for handle in thread_handles {
      let detected_by_thread = handle.join().unwrap();
      detected_list.extend(detected_by_thread);
    }

    // Overwrite self.detected_list with the new list
    detected_list
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
