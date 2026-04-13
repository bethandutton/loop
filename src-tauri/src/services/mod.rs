use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::Mutex;
use portable_pty::{CommandBuilder, PtySize, native_pty_system};

struct ServiceProcess {
    #[allow(dead_code)]
    script_name: String,
    writer: Box<dyn Write + Send>,
    child: Box<dyn portable_pty::Child + Send>,
    buffer: Vec<u8>,
    state: String, // "running", "stopped", "errored"
}

pub struct ServiceManager {
    services: Mutex<HashMap<String, ServiceProcess>>,
}

#[derive(Clone, serde::Serialize)]
pub struct ServiceDef {
    pub name: String,
    pub command: String,
}

#[derive(Clone, serde::Serialize)]
pub struct ServiceStatus {
    pub id: String,
    pub name: String,
    pub state: String,
}

impl ServiceManager {
    pub fn new() -> Self {
        ServiceManager {
            services: Mutex::new(HashMap::new()),
        }
    }

    pub fn start_service(
        &self,
        service_id: &str,
        script_name: &str,
        worktree_path: &str,
    ) -> Result<(), String> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize { rows: 24, cols: 80, pixel_width: 0, pixel_height: 0 })
            .map_err(|e| format!("Failed to open PTY: {}", e))?;

        let mut cmd = CommandBuilder::new("npm");
        cmd.args(["run", script_name]);
        cmd.cwd(worktree_path);
        cmd.env("TERM", "xterm-256color");

        let child = pair.slave.spawn_command(cmd)
            .map_err(|e| format!("Failed to spawn service: {}", e))?;

        let writer = pair.master.take_writer()
            .map_err(|e| format!("Failed to get writer: {}", e))?;

        let mut reader = pair.master.try_clone_reader()
            .map_err(|e| format!("Failed to get reader: {}", e))?;

        let sid = service_id.to_string();
        let services_ref = &self.services;

        // We can't easily share the mutex with the thread, so we'll buffer in a separate Arc
        let buffer_arc = std::sync::Arc::new(Mutex::new(Vec::<u8>::new()));
        let buffer_clone = buffer_arc.clone();

        std::thread::spawn(move || {
            let mut chunk = [0u8; 4096];
            loop {
                match reader.read(&mut chunk) {
                    Ok(0) => break,
                    Ok(n) => {
                        if let Ok(mut buf) = buffer_clone.lock() {
                            buf.extend_from_slice(&chunk[..n]);
                            // Cap at 100KB
                            if buf.len() > 100_000 {
                                let excess = buf.len() - 100_000;
                                buf.drain(..excess);
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        let process = ServiceProcess {
            script_name: script_name.to_string(),
            writer,
            child,
            buffer: Vec::new(), // We'll read from buffer_arc on demand
            state: "running".to_string(),
        };

        self.services.lock()
            .map_err(|e| format!("Lock error: {}", e))?
            .insert(service_id.to_string(), process);

        Ok(())
    }

    pub fn stop_service(&self, service_id: &str) -> Result<(), String> {
        let mut services = self.services.lock()
            .map_err(|e| format!("Lock error: {}", e))?;

        if let Some(mut svc) = services.remove(service_id) {
            let _ = svc.child.kill();
            let _ = svc.child.wait();
        }
        Ok(())
    }

    pub fn stop_all(&self) -> Result<(), String> {
        let mut services = self.services.lock()
            .map_err(|e| format!("Lock error: {}", e))?;

        for (_, mut svc) in services.drain() {
            let _ = svc.child.kill();
            let _ = svc.child.wait();
        }
        Ok(())
    }

    pub fn list_running(&self) -> Vec<ServiceStatus> {
        if let Ok(services) = self.services.lock() {
            services.iter().map(|(id, svc)| ServiceStatus {
                id: id.clone(),
                name: svc.script_name.clone(),
                state: svc.state.clone(),
            }).collect()
        } else {
            vec![]
        }
    }
}

/// Detect available scripts from package.json in a directory
pub fn detect_scripts(worktree_path: &str) -> Result<Vec<ServiceDef>, String> {
    let pkg_path = std::path::Path::new(worktree_path).join("package.json");
    if !pkg_path.exists() {
        return Ok(vec![]);
    }

    let content = std::fs::read_to_string(&pkg_path)
        .map_err(|e| format!("Failed to read package.json: {}", e))?;

    let pkg: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse package.json: {}", e))?;

    let scripts = pkg.get("scripts")
        .and_then(|s| s.as_object())
        .map(|scripts| {
            scripts.iter().map(|(name, cmd)| ServiceDef {
                name: name.clone(),
                command: cmd.as_str().unwrap_or("").to_string(),
            }).collect()
        })
        .unwrap_or_default();

    Ok(scripts)
}
