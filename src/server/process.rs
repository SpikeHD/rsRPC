use sysinfo::SystemExt;

pub fn process_list() -> Vec<String> {
    let mut processes = Vec::new();
    let sys = sysinfo::System::new_all();
    
    for (pid, process) in sys.get_processes() {
        processes.push(process.name());
    }

    processes
}