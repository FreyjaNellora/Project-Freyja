// Engine child process management.
//
// Spawns freyja engine as a child process, writes commands to stdin,
// reads stdout line-by-line in a background thread, and emits
// Tauri events to the frontend for each line.

use std::io::{BufRead, BufReader, BufWriter, Write};
use std::process::{Child, Command, Stdio};
use std::thread;
use tauri::Emitter;

/// Payload for engine-output events, tagged with the engine generation
/// so the frontend can discard stale output from a killed process.
#[derive(Clone, serde::Serialize)]
pub struct EngineOutputPayload {
    pub line: String,
    pub gen: u64,
}

/// Manages the lifecycle of the engine child process.
pub struct EngineManager {
    child: Option<Child>,
    stdin: Option<BufWriter<std::process::ChildStdin>>,
    /// Monotonically increasing generation counter. Bumped on every spawn()
    /// so the reader thread from a killed engine emits events with a stale
    /// generation that the frontend can filter out.
    generation: u64,
}

impl EngineManager {
    pub fn new() -> Self {
        Self {
            child: None,
            stdin: None,
            generation: 0,
        }
    }

    /// Spawn the engine process and start the stdout reader thread.
    /// Returns the engine generation number for stale-event filtering.
    pub fn spawn(&mut self, app: tauri::AppHandle) -> Result<u64, String> {
        // Kill any existing engine first
        if self.child.is_some() {
            self.kill()?;
        }

        self.generation += 1;
        let gen = self.generation;

        let engine_path = Self::resolve_engine_path()?;

        let mut child = Command::new(&engine_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn engine at {}: {}", engine_path, e))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "Failed to capture engine stdin".to_string())?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "Failed to capture engine stdout".to_string())?;

        // Spawn a thread to read stdout line-by-line and emit events.
        // The thread captures `gen` so every event is tagged with the engine
        // instance that produced it. When the engine is killed and respawned,
        // a new generation is assigned, and the frontend ignores events
        // from the old reader thread that are still draining the pipe.
        let app_handle = app.clone();
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        let _ =
                            app_handle.emit("engine-output", EngineOutputPayload { line, gen });
                    }
                    Err(_) => break,
                }
            }
            // Engine stdout closed — process likely exited
            let _ = app_handle.emit("engine-exit", gen);
        });

        self.stdin = Some(BufWriter::new(stdin));
        self.child = Some(child);

        Ok(gen)
    }

    /// Send a command line to the engine's stdin.
    pub fn send_command(&mut self, cmd: &str) -> Result<(), String> {
        let stdin = self
            .stdin
            .as_mut()
            .ok_or_else(|| "Engine not running".to_string())?;

        writeln!(stdin, "{}", cmd)
            .map_err(|e| format!("Failed to write to engine stdin: {}", e))?;
        stdin
            .flush()
            .map_err(|e| format!("Failed to flush engine stdin: {}", e))?;

        Ok(())
    }

    /// Kill the engine child process.
    pub fn kill(&mut self) -> Result<(), String> {
        // Drop stdin first to signal the engine
        self.stdin = None;

        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }

        Ok(())
    }

    /// Resolve the path to the freyja engine binary.
    fn resolve_engine_path() -> Result<String, String> {
        // Development mode: look for the engine binary relative to the project
        let dev_paths = [
            // Cargo workspace: binary in project root target/
            // From freyja-ui/src-tauri/ (where the Tauri binary runs during dev)
            "../../target/debug/freyja.exe",
            "../../target/debug/freyja",
            "../../target/release/freyja.exe",
            "../../target/release/freyja",
            // From project root
            "target/debug/freyja.exe",
            "target/debug/freyja",
            "target/release/freyja.exe",
            "target/release/freyja",
        ];

        for path in &dev_paths {
            let p = std::path::Path::new(path);
            if p.exists() {
                return p
                    .canonicalize()
                    .map(|p| p.to_string_lossy().to_string())
                    .map_err(|e| format!("Failed to resolve engine path: {}", e));
            }
        }

        Err(format!(
            "Could not find freyja engine binary. Searched: {:?}. Run `cargo build` in project root first.",
            dev_paths
        ))
    }
}

impl Drop for EngineManager {
    fn drop(&mut self) {
        let _ = self.kill();
    }
}
