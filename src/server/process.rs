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

  // ms to wait in between each process scan
  pub scan_wait_ms: u64,
  pub detectable_list: Vec<DetectableActivity>,
  pub event_sender: mpsc::Sender<ProcessDetectedEvent>,
}

impl ProcessServer {
  pub fn new(
    detectable: Vec<DetectableActivity>,
    event_sender: mpsc::Sender<ProcessDetectedEvent>,
  ) -> Self {
    ProcessServer {
      detected_list: Arc::new(Mutex::new(vec![])),
      scan_wait_ms: 1,
      detectable_list: detectable,
      event_sender,
    }
  }

  pub fn append_detectables(&mut self, detectable: Vec<DetectableActivity>) {
    self.detectable_list.extend(detectable);
  }

  pub fn start(mut self) {
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

  pub fn scan_for_processes(&mut self) -> Vec<DetectableActivity> {
    logger::log("Process scan triggered");
    let processes = ProcessServer::process_list();
    let mut detected_list = vec![];

    for obj in &self.detectable_list {
      // if executables is null, just skip
      if obj.executables.is_none() {
        continue;
      }
      
      // It's fine if this is a little slow so as to not crank the CPU
      std::thread::sleep(std::time::Duration::from_millis(self.scan_wait_ms));

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

            detected_list.push(new_activity);
          }
        }
      }
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
