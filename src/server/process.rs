use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::vec;

use sysinfo::SystemExt;
use sysinfo::ProcessExt;

use super::super::DetectableActivity;

#[derive(Clone)]
pub struct Exec {
  pid: u64,
  name: String,
}

pub struct ProcessServer {
  detected_list: Arc<Mutex<Vec<DetectableActivity>>>,

  // ms to wait in between each process scan
  pub scan_wait_ms: u64,
  pub detectable_list: Vec<DetectableActivity>,
}

impl ProcessServer {
  pub fn new(detectable: Vec<DetectableActivity>) -> Self {
    ProcessServer {
      detected_list: Arc::new(Mutex::new(vec![])),
      scan_wait_ms: 1,
      detectable_list: detectable,
    }
  }

  pub fn start(mut self) {
    std::thread::spawn(move || {
      // Run the process scan repeatedly (every 3 seconds)
      loop {
        let detected = self.scan_for_processes();

        // If the detected list has changed, send a message to the main thread
        for activity in &detected {
          // Check for matching name properties
          let mut found = false;
          let detected = self.detected_list.lock().unwrap();

          for detected_activity in detected.iter() {
            if detected_activity.name == activity.name {
              found = true;
              break;
            }
          }

          if !found {
            // TODO: if anything changes, pass message to the main thread
          }
        }

        std::thread::sleep(Duration::from_secs(3));
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
    println!("Process scan triggered");
    let processes = ProcessServer::process_list();
    let mut detected_list = vec![];

    for process in processes {
      // It's fine if this is a little slow so as to not crank the CPU
      std::thread::sleep(std::time::Duration::from_millis(self.scan_wait_ms));

      let mut possibilities = vec![process.clone()];

      // It also could have no extension
      if process.name.contains(".") {
        // Split the name by the dot
        let split: Vec<&str> = process.name.split(".").collect();

        // New exec struct with name not having extension
        let mut new_exec = process.clone();
        new_exec.name = split[0].to_string();

        // Push the new exec struct
        possibilities.push(new_exec);
      }

      for obj in &self.detectable_list {
        // if executables is null, just skip
        if obj.executables.is_none() {
          continue;
        }
        
        // detectable['executables'] is an array of objects with keys is_launcher, name, and os
        for executable in obj.executables.as_ref().unwrap() {
          // Check each possibility
          for possibility in &possibilities {
            // If this game is not in the list of already detected games, and the executable name matches, add
            if executable.name == *possibility.name {
              // Push the whole game
              let mut new_activity = obj.clone();
              new_activity.pid = Some(process.pid);
              detected_list.push(obj.clone());
            }
          }
        }
      }
    }

    // Processes found:
    for activity in &detected_list {
      println!(r#"
      {{
        "cmd": "SET_ACTIVITY",
        "args": {{
          "activity": {{
            "application_id": {},
            "name": "{}",
            "timestamps": {{
              start: "{}"
            }}
          }},
          "pid": "{}"
        }}
      }}
      "#, activity.id, activity.name, chrono::Utc::now().to_rfc3339(), activity.pid.unwrap_or_default());
    }

    // Overwrite self.detected_list with the new list
    detected_list
  }
}