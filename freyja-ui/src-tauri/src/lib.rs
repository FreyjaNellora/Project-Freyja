// Freyja UI — Tauri v2 backend
//
// Manages the engine child process and bridges IPC between
// the React frontend and the Freyja Protocol (stdin/stdout).

mod engine;

use engine::EngineManager;
use std::sync::Mutex;
use tauri::State;

/// Application state shared across IPC commands.
struct AppState {
    engine: EngineManager,
}

/// Spawn the engine child process. Returns the engine generation number.
#[tauri::command]
fn spawn_engine(state: State<Mutex<AppState>>, app: tauri::AppHandle) -> Result<u64, String> {
    let mut state = state.lock().map_err(|e| e.to_string())?;
    state.engine.spawn(app)
}

/// Send a command string to the engine's stdin.
#[tauri::command]
fn send_command(cmd: String, state: State<Mutex<AppState>>) -> Result<(), String> {
    let mut state = state.lock().map_err(|e| e.to_string())?;
    state.engine.send_command(&cmd)
}

/// Kill the engine child process.
#[tauri::command]
fn kill_engine(state: State<Mutex<AppState>>) -> Result<(), String> {
    let mut state = state.lock().map_err(|e| e.to_string())?;
    state.engine.kill()
}

/// Run the Tauri application.
pub fn run() {
    tauri::Builder::default()
        .manage(Mutex::new(AppState {
            engine: EngineManager::new(),
        }))
        .invoke_handler(tauri::generate_handler![
            spawn_engine,
            send_command,
            kill_engine,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Freyja UI");
}
