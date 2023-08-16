use std::time::Duration;
use std::vec;

use serde_json::Value;
use sysinfo::SystemExt;
use sysinfo::ProcessExt;

use super::base::BaseServer;
use super::super::DetectableActivity;

pub struct ProcessServer {
  base: BaseServer,
  detected_list: Vec<DetectableActivity>,

  // ms to wait in between each process scan
  pub scan_wait_ms: u64,
  pub detectable_list: Vec<DetectableActivity>,
}

impl ProcessServer {
  pub fn new(detectable: Vec<DetectableActivity>) -> Self {
    ProcessServer {
      base: BaseServer::new(),
      detected_list: vec![],
      scan_wait_ms: 1,
      detectable_list: detectable,
    }
  }

  pub fn start(&mut self) {
    // Run the process scan repeatedly (every 3 seconds)
    loop {
      let detected = self.scan_for_processes();

      // If the detected list has changed, send a message to the main thread
      for activity in &detected {
        // Check for matching name properties
        let mut found = false;

        for detected_activity in &self.detected_list {
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
  }

  pub fn process_list() -> Vec<String> {
    let mut processes = Vec::new();
    let sys = sysinfo::System::new_all();
      
    for (_pid, process) in sys.processes() {
      processes.push(process.name().to_string());
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
      if process.contains(".") {
        possibilities.push(process.split(".").collect::<Vec<&str>>()[0].to_string());
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
            if executable.name == *possibility {
              // Push the whole game
              detected_list.push(obj.clone());
            }
          }
        }
      }
    }

    // Processes found:
    for activity in &detected_list {
      println!("Found process: {}", activity.name);
    }

    // Overwrite self.detected_list with the new list
    detected_list
  }
}