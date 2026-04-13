use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::Emitter;

const BUFFER_MAX_LINES: usize = 10_000;

struct ScrollbackBuffer {
    data: Vec<u8>,
    file_path: String,
}

impl ScrollbackBuffer {
    fn new(file_path: &str) -> Self {
        // Ensure parent dir exists
        if let Some(parent) = std::path::Path::new(file_path).parent() {
            std::fs::create_dir_all(parent).ok();
        }
        ScrollbackBuffer {
            data: Vec::new(),
            file_path: file_path.to_string(),
        }
    }

    fn append(&mut self, chunk: &[u8]) {
        self.data.extend_from_slice(chunk);
        // Trim to max lines
        let newline_count = self.data.iter().filter(|&&b| b == b'\n').count();
        if newline_count > BUFFER_MAX_LINES {
            let excess = newline_count - BUFFER_MAX_LINES;
            let mut skipped = 0;
            let mut pos = 0;
            for (i, &b) in self.data.iter().enumerate() {
                if b == b'\n' {
                    skipped += 1;
                    if skipped >= excess {
                        pos = i + 1;
                        break;
                    }
                }
            }
            self.data.drain(..pos);
        }
        // Also write to disk
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file_path)
        {
            let _ = file.write_all(chunk);
        }
    }

    fn get_all(&self) -> Vec<u8> {
        self.data.clone()
    }
}

struct Session {
    #[allow(dead_code)]
    ticket_id: String,
    writer: Box<dyn Write + Send>,
    #[allow(dead_code)]
    child: Box<dyn portable_pty::Child + Send>,
    buffer: Arc<Mutex<ScrollbackBuffer>>,
    unread: Arc<AtomicBool>,
}

pub struct SessionManager {
    sessions: Mutex<HashMap<String, Session>>,
}

impl SessionManager {
    pub fn new() -> Self {
        SessionManager {
            sessions: Mutex::new(HashMap::new()),
        }
    }

    pub fn spawn_session(
        &self,
        session_id: &str,
        ticket_id: &str,
        worktree_path: &str,
        claude_path: &str,
        scrollback_path: &str,
        app_handle: tauri::AppHandle,
    ) -> Result<(), String> {
        let pty_system = native_pty_system();

        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("Failed to open PTY: {}", e))?;

        let mut cmd = CommandBuilder::new(claude_path);
        cmd.cwd(worktree_path);
        // Set TERM for proper terminal behavior
        cmd.env("TERM", "xterm-256color");

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| format!("Failed to spawn Claude Code: {}", e))?;

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| format!("Failed to get PTY writer: {}", e))?;

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| format!("Failed to get PTY reader: {}", e))?;

        let buffer = Arc::new(Mutex::new(ScrollbackBuffer::new(scrollback_path)));
        let unread = Arc::new(AtomicBool::new(false));

        let buffer_clone = buffer.clone();
        let unread_clone = unread.clone();
        let sid = session_id.to_string();

        // Reader thread: reads PTY output, buffers it, emits events
        std::thread::spawn(move || {
            let mut chunk = [0u8; 4096];
            loop {
                match reader.read(&mut chunk) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        let data = &chunk[..n];
                        if let Ok(mut buf) = buffer_clone.lock() {
                            buf.append(data);
                        }
                        unread_clone.store(true, Ordering::Relaxed);

                        // Emit event to frontend
                        let event_name = format!("session_output_{}", sid);
                        let _ = app_handle.emit(&event_name, data.to_vec());
                    }
                    Err(_) => break,
                }
            }
        });

        let session = Session {
            ticket_id: ticket_id.to_string(),
            writer,
            child,
            buffer,
            unread,
        };

        self.sessions
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?
            .insert(session_id.to_string(), session);

        Ok(())
    }

    pub fn write_to_session(&self, session_id: &str, data: &[u8]) -> Result<(), String> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;

        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("Session {} not found", session_id))?;

        session
            .writer
            .write_all(data)
            .map_err(|e| format!("Failed to write to PTY: {}", e))?;

        session
            .writer
            .flush()
            .map_err(|e| format!("Failed to flush PTY: {}", e))?;

        Ok(())
    }

    pub fn get_scrollback(&self, session_id: &str) -> Result<Vec<u8>, String> {
        let sessions = self
            .sessions
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;

        let session = sessions
            .get(session_id)
            .ok_or_else(|| format!("Session {} not found", session_id))?;

        // Mark as read
        session.unread.store(false, Ordering::Relaxed);

        let buf = session
            .buffer
            .lock()
            .map_err(|e| format!("Buffer lock error: {}", e))?;

        Ok(buf.get_all())
    }

    pub fn kill_session(&self, session_id: &str) -> Result<(), String> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;

        if let Some(mut session) = sessions.remove(session_id) {
            let _ = session.child.kill();
            let _ = session.child.wait();
        }

        Ok(())
    }

    pub fn is_unread(&self, session_id: &str) -> bool {
        if let Ok(sessions) = self.sessions.lock() {
            if let Some(session) = sessions.get(session_id) {
                return session.unread.load(Ordering::Relaxed);
            }
        }
        false
    }

    pub fn has_session(&self, session_id: &str) -> bool {
        self.sessions
            .lock()
            .map(|s| s.contains_key(session_id))
            .unwrap_or(false)
    }

    pub fn resize_session(&self, session_id: &str, rows: u16, cols: u16) -> Result<(), String> {
        // Resizing requires the master PTY which we don't store currently
        // This is a no-op for now; xterm.js will handle its own viewport
        let _ = (session_id, rows, cols);
        Ok(())
    }
}
