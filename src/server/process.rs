use std::vec;

use serde_json::Value;
use sysinfo::SystemExt;
use sysinfo::ProcessExt;

use super::base::BaseServer;
use super::super::DetectableActivity;

pub struct ProcessServer {
  base: BaseServer,
  detected_list: Vec<String>,

  // ms to wait in between each process scan
  pub scan_wait_ms: u64,
}

impl ProcessServer {
  pub fn new() -> Self {
    ProcessServer {
      base: BaseServer::new(),
      detected_list: vec![],
      scan_wait_ms: 1,
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

  pub fn scan_for_processes(mut self, detectable: &Vec<DetectableActivity>) {
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

      for obj in detectable {
        // if executables is null, just skip
        if obj.executables.is_none() {
          continue;
        }
        
        // detectable['executables'] is an array of objects with keys is_launcher, name, and os
        for executable in obj.executables.as_ref().unwrap() {
          // Check each possibility
          for possibility in &possibilities {
            // If this game is not in the list of already detected games, and the executable name matches, add
            if executable.name == *possibility && !self.detected_list.contains(&possibility) {
              detected_list.push(possibility.to_string());
            }
          }
        }
      }
    }

    // Processes found:
    for process in &detected_list {
      println!("Found process: {}", process);
    }

    // Overwrite self.detected_list with the new list
    self.detected_list = detected_list;
  }
}