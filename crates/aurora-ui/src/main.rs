//! AuroraHeart IDE - Tauri Backend
//!
//! This is the Tauri backend for AuroraHeart IDE, providing commands for
//! file operations, AI agent integration, and credential management.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod terminal;

use aurora_agent::{AgenticEvent, AnthropicClient, Conversation, ToolExecutor};
use aurora_core::{
    detect_language, find_project_root, get_project_name, read_file, write_file, Config,
    ConfigError, CredentialStore,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::{Manager, State};
use terminal::{ShellType, TerminalId, TerminalManager};

/// Initialize tracing for logging
fn init_tracing() {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "aurora=debug,info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

/// File tree item for serialization to frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTreeItem {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
}

/// File open result
#[derive(Debug, Serialize)]
pub struct FileOpenResult {
    pub path: String,
    pub content: String,
}

/// Application state shared across Tauri commands
pub struct AppState {
    pub project_root: Arc<Mutex<PathBuf>>,
    pub conversation: Arc<Mutex<Conversation>>,
    pub terminal_manager: TerminalManager,
}

/// Load files from current directory into file tree
fn load_file_tree_internal<P: AsRef<Path>>(dir: P) -> Vec<FileTreeItem> {
    let mut items = Vec::new();

    if let Ok(entries) = std::fs::read_dir(dir.as_ref()) {
        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            // Skip hidden files and certain directories
            if file_name.starts_with('.') || file_name == "target" || file_name == "node_modules" {
                continue;
            }

            let is_directory = path.is_dir();
            let path_str = path.to_string_lossy().to_string();

            items.push(FileTreeItem {
                name: file_name,
                path: path_str,
                is_directory,
            });
        }
    }

    // Sort: directories first, then files, both alphabetically
    items.sort_by(|a, b| match (a.is_directory, b.is_directory) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    items
}

/// Mask API key for display (show first 7 chars and last 4 chars)
fn mask_api_key(api_key: &str) -> String {
    if api_key.len() <= 11 {
        return "****".to_string();
    }

    let prefix = &api_key[..7]; // "sk-ant-"
    let suffix = &api_key[api_key.len() - 4..];
    format!("{}...{}", prefix, suffix)
}

// ============================================================================
// TAURI COMMANDS
// ============================================================================

/// Get file tree for the project root
#[tauri::command]
async fn get_file_tree(state: State<'_, AppState>) -> Result<Vec<FileTreeItem>, String> {
    tracing::info!("get_file_tree command called");
    let project_root = state.project_root.lock().unwrap();
    Ok(load_file_tree_internal(&*project_root))
}

/// Get directory contents
#[tauri::command]
async fn get_directory_contents(path: String) -> Result<Vec<FileTreeItem>, String> {
    tracing::info!("get_directory_contents command called for: {}", path);
    Ok(load_file_tree_internal(path))
}

/// Open a file using native file dialog
#[tauri::command]
async fn open_file(app: tauri::AppHandle) -> Result<Option<FileOpenResult>, String> {
    use tauri_plugin_dialog::DialogExt;

    tracing::info!("open_file command called");

    // Use Tauri v2 dialog plugin
    let file_path = app
        .dialog()
        .file()
        .blocking_pick_file();

    if let Some(path) = file_path {
        // FilePath::as_path() returns Option<&Path>, so we need to handle it
        if let Some(p) = path.as_path() {
            let path_str = p.to_string_lossy().to_string();
            match read_file(&path_str) {
                Ok(content) => {
                    tracing::info!("Opened file: {}", path_str);
                    Ok(Some(FileOpenResult {
                        path: path_str,
                        content,
                    }))
                }
                Err(e) => {
                    let error_msg = format!("Error reading file: {}", e);
                    tracing::error!("{}", error_msg);
                    Err(error_msg)
                }
            }
        } else {
            Err("Invalid file path".to_string())
        }
    } else {
        // User cancelled the dialog
        Ok(None)
    }
}

/// Read a file by path
#[tauri::command]
async fn read_file_by_path(path: String) -> Result<String, String> {
    tracing::info!("read_file_by_path command called for: {}", path);

    read_file(&path).map_err(|e| {
        let error_msg = format!("Error reading file: {}", e);
        tracing::error!("{}", error_msg);
        error_msg
    })
}

/// Save file content to disk
#[tauri::command]
async fn save_file(path: String, content: String) -> Result<(), String> {
    tracing::info!("save_file command called for: {}", path);

    write_file(&path, &content).map_err(|e| {
        let error_msg = format!("Error saving file: {}", e);
        tracing::error!("{}", error_msg);
        error_msg
    })?;

    tracing::info!("File saved successfully: {}", path);
    Ok(())
}

/// Save API key to encrypted credential store
#[tauri::command]
async fn save_api_key(key: String, state: State<'_, AppState>) -> Result<(), String> {
    tracing::info!("save_api_key command called");

    let project_root = state.project_root.lock().unwrap();
    let store = CredentialStore::for_project(&*project_root);
    store
        .store("anthropic_api_key", &key, "auroraheart")
        .map_err(|e| {
            let error_msg = format!("Failed to save API key: {}", e);
            tracing::error!("{}", error_msg);
            error_msg
        })?;

    tracing::info!("API key saved successfully");
    Ok(())
}

/// Load API key from encrypted credential store
#[tauri::command]
async fn load_api_key(state: State<'_, AppState>) -> Result<String, String> {
    tracing::info!("load_api_key command called");

    let project_root = state.project_root.lock().unwrap();
    let store = CredentialStore::for_project(&*project_root);
    store
        .retrieve("anthropic_api_key", "auroraheart")
        .map_err(|e| {
            tracing::debug!("No API key found: {}", e);
            "No API key configured".to_string()
        })
}

/// Send a message to Claude and run the agentic loop
#[tauri::command]
async fn send_message(message: String, state: State<'_, AppState>) -> Result<String, String> {
    tracing::info!("send_message command called: {}", message);

    // Load API key
    let api_key = {
        let project_root = state.project_root.lock().unwrap();
        let store = CredentialStore::for_project(&*project_root);
        store
            .retrieve("anthropic_api_key", "auroraheart")
            .map_err(|e| {
                let error_msg = "⚠ No API key configured. Please set your API key in Settings.";
                tracing::error!("Failed to load API key: {}", e);
                error_msg.to_string()
            })?
    };

    // Add user message to conversation and truncate if needed
    {
        let mut conv = state.conversation.lock().unwrap();
        conv.add_user_message(&message);
        // Keep conversation within reasonable limits (50k tokens = ~200k chars)
        let removed = conv.truncate_to_tokens(50_000);
        if removed > 0 {
            tracing::info!("Truncated {} old messages from conversation", removed);
        }
    }

    // Create client
    let client = AnthropicClient::new(api_key);

    // Create tool executor
    let project_root_path = {
        let guard = state.project_root.lock().unwrap();
        guard.clone()
    };
    let executor = ToolExecutor::with_working_directory(project_root_path);

    // Clone conversation for agentic loop
    let mut conv = {
        let guard = state.conversation.lock().unwrap();
        guard.clone()
    };

    // Run agentic loop
    let events = client
        .run_agentic_loop(&mut conv, &executor, None)
        .await
        .map_err(|e| {
            let error_msg = format!("⚠ Error: {}", e);
            tracing::error!("Agentic loop error: {:?}", e);
            error_msg
        })?;

    // Update conversation with the modified version
    {
        let mut conversation_lock = state.conversation.lock().unwrap();
        *conversation_lock = conv;
    }

    // Format events into response text
    let mut output = String::new();
    let mut final_text = String::new();

    for event in &events {
        match event {
            AgenticEvent::ToolCall { id, name, input } => {
                let tool_info = format!("\n[🔧 Tool: {} (id: {})]\n", name, id);
                output.push_str(&tool_info);
                tracing::info!("Tool call: {} with input: {:?}", name, input);
            }
            AgenticEvent::ToolResult {
                tool_use_id,
                content,
                is_error,
            } => {
                let result_prefix = if is_error == &Some(true) {
                    "❌ Error"
                } else {
                    "✓ Result"
                };
                // Truncate long tool results for display
                let display_content = if content.len() > 200 {
                    format!("{}... ({} chars)", &content[..200], content.len())
                } else {
                    content.clone()
                };
                let tool_result = format!("[{}: {}]\n", result_prefix, display_content);
                output.push_str(&tool_result);
                tracing::info!(
                    "Tool result for {}: {} chars",
                    tool_use_id,
                    content.len()
                );
            }
            AgenticEvent::TextResponse { text } => {
                final_text.push_str(text);
            }
        }
    }

    // Add final text response
    output.push_str(&final_text);

    tracing::info!("Agentic loop completed with {} events", events.len());
    Ok(output)
}

/// Clear the conversation history
#[tauri::command]
async fn clear_chat(state: State<'_, AppState>) -> Result<(), String> {
    tracing::info!("clear_chat command called");

    let mut conv = state.conversation.lock().unwrap();
    conv.clear();

    tracing::info!("Conversation cleared");
    Ok(())
}

// ============================================================================
// TERMINAL COMMANDS
// ============================================================================

/// Get list of available shells on the system
#[tauri::command]
async fn get_available_shells() -> Result<Vec<ShellType>, String> {
    tracing::info!("get_available_shells command called");
    Ok(TerminalManager::detect_available_shells())
}

/// Get the default shell for the platform
#[tauri::command]
async fn get_default_shell() -> Result<ShellType, String> {
    tracing::info!("get_default_shell command called");
    Ok(TerminalManager::default_shell())
}

/// Spawn a new terminal session
#[tauri::command]
async fn spawn_terminal(
    shell_type: ShellType,
    cols: u16,
    rows: u16,
    state: State<'_, AppState>,
) -> Result<TerminalId, String> {
    tracing::info!(
        "spawn_terminal command called: {:?} ({}x{})",
        shell_type,
        cols,
        rows
    );

    // Use project root as working directory
    let project_root = state.project_root.lock().unwrap();
    let working_dir = project_root.to_string_lossy().to_string();

    state
        .terminal_manager
        .spawn_terminal(shell_type, cols, rows, Some(working_dir))
        .map_err(|e| {
            let error_msg = format!("Failed to spawn terminal: {}", e);
            tracing::error!("{}", error_msg);
            error_msg
        })
}

/// Write data to terminal
#[tauri::command]
async fn write_terminal(
    id: TerminalId,
    data: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    tracing::debug!("write_terminal command called: {} ({} bytes)", id, data.len());

    state.terminal_manager.write_terminal(&id, &data).map_err(|e| {
        let error_msg = format!("Failed to write to terminal: {}", e);
        tracing::error!("{}", error_msg);
        error_msg
    })
}

/// Resize terminal
#[tauri::command]
async fn resize_terminal(
    id: TerminalId,
    cols: u16,
    rows: u16,
    state: State<'_, AppState>,
) -> Result<(), String> {
    tracing::info!("resize_terminal command called: {} ({}x{})", id, cols, rows);

    state
        .terminal_manager
        .resize_terminal(&id, cols, rows)
        .map_err(|e| {
            let error_msg = format!("Failed to resize terminal: {}", e);
            tracing::error!("{}", error_msg);
            error_msg
        })
}

/// Close terminal session
#[tauri::command]
async fn close_terminal(id: TerminalId, state: State<'_, AppState>) -> Result<(), String> {
    tracing::info!("close_terminal command called: {}", id);

    state.terminal_manager.close_terminal(&id).map_err(|e| {
        let error_msg = format!("Failed to close terminal: {}", e);
        tracing::error!("{}", error_msg);
        error_msg
    })
}

/// List all active terminal IDs
#[tauri::command]
async fn list_terminals(state: State<'_, AppState>) -> Result<Vec<TerminalId>, String> {
    tracing::info!("list_terminals command called");
    Ok(state.terminal_manager.list_terminals())
}

// ============================================================================
// GIT COMMANDS
// ============================================================================

/// Git status information
#[derive(Debug, Serialize, Deserialize)]
pub struct GitStatus {
    pub branch: Option<String>,
    pub modified: Vec<String>,
    pub staged: Vec<String>,
    pub untracked: Vec<String>,
    pub ahead: usize,
    pub behind: usize,
}

/// Get git status for the project
#[tauri::command]
async fn get_git_status(state: State<'_, AppState>) -> Result<GitStatus, String> {
    tracing::info!("get_git_status command called");

    let project_root = state.project_root.lock().unwrap().clone();

    // Get current branch
    let branch_output = std::process::Command::new("git")
        .args(&["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(&project_root)
        .output();

    let branch = match branch_output {
        Ok(output) if output.status.success() => {
            Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
        }
        _ => None,
    }
    .filter(|s| !s.is_empty());

    // Get status
    let status_output = std::process::Command::new("git")
        .args(&["status", "--porcelain"])
        .current_dir(&project_root)
        .output()
        .map_err(|e| format!("Failed to run git status: {}", e))?;

    let mut modified = Vec::new();
    let mut staged = Vec::new();
    let mut untracked = Vec::new();

    if status_output.status.success() {
        let status_str = String::from_utf8_lossy(&status_output.stdout);
        for line in status_str.lines() {
            if line.len() < 3 {
                continue;
            }
            let status_code = &line[..2];
            let file_path = line[3..].to_string();

            match status_code {
                "??" => untracked.push(file_path),
                " M" | " D" => modified.push(file_path),
                "M " | "A " | "D " | "R " | "C " => staged.push(file_path),
                "MM" | "AM" | "AD" => {
                    staged.push(file_path.clone());
                    modified.push(file_path);
                }
                _ => {}
            }
        }
    }

    // Get ahead/behind count
    let ahead_behind_output = std::process::Command::new("git")
        .args(&["rev-list", "--left-right", "--count", "HEAD...@{u}"])
        .current_dir(&project_root)
        .output();

    let (ahead, behind) = match ahead_behind_output {
        Ok(output) if output.status.success() => {
            let counts = String::from_utf8_lossy(&output.stdout);
            let parts: Vec<&str> = counts.trim().split_whitespace().collect();
            if parts.len() == 2 {
                (
                    parts[0].parse().unwrap_or(0),
                    parts[1].parse().unwrap_or(0),
                )
            } else {
                (0, 0)
            }
        }
        _ => (0, 0),
    };

    Ok(GitStatus {
        branch,
        modified,
        staged,
        untracked,
        ahead,
        behind,
    })
}

/// Check if a file has git modifications
#[tauri::command]
async fn is_file_modified(path: String, state: State<'_, AppState>) -> Result<bool, String> {
    let project_root = state.project_root.lock().unwrap().clone();

    let output = std::process::Command::new("git")
        .args(&["status", "--porcelain", &path])
        .current_dir(project_root)
        .output()
        .map_err(|e| format!("Failed to check git status: {}", e))?;

    Ok(output.status.success() && !output.stdout.is_empty())
}

/// Stage files for commit
#[tauri::command]
async fn git_stage(files: Vec<String>, state: State<'_, AppState>) -> Result<(), String> {
    tracing::info!("git_stage command called for {} files", files.len());
    let project_root = state.project_root.lock().unwrap().clone();

    for file in &files {
        let output = std::process::Command::new("git")
            .args(&["add", file])
            .current_dir(&project_root)
            .output()
            .map_err(|e| format!("Failed to stage file: {}", e))?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to stage {}: {}", file, error));
        }
    }

    Ok(())
}

/// Unstage files
#[tauri::command]
async fn git_unstage(files: Vec<String>, state: State<'_, AppState>) -> Result<(), String> {
    tracing::info!("git_unstage command called for {} files", files.len());
    let project_root = state.project_root.lock().unwrap().clone();

    for file in &files {
        let output = std::process::Command::new("git")
            .args(&["restore", "--staged", file])
            .current_dir(&project_root)
            .output()
            .map_err(|e| format!("Failed to unstage file: {}", e))?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to unstage {}: {}", file, error));
        }
    }

    Ok(())
}

/// Create a git commit
#[tauri::command]
async fn git_commit(message: String, amend: bool, state: State<'_, AppState>) -> Result<String, String> {
    tracing::info!("git_commit command called: amend={}", amend);
    let project_root = state.project_root.lock().unwrap().clone();

    let mut args = vec!["commit", "-m", &message];
    if amend {
        args.push("--amend");
    }

    let output = std::process::Command::new("git")
        .args(&args)
        .current_dir(project_root)
        .output()
        .map_err(|e| format!("Failed to create commit: {}", e))?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Commit failed: {}", error));
    }

    let result = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(result)
}

/// Push commits to remote
#[tauri::command]
async fn git_push(remote: Option<String>, branch: Option<String>, state: State<'_, AppState>) -> Result<String, String> {
    tracing::info!("git_push command called");
    let project_root = state.project_root.lock().unwrap().clone();

    let mut args = vec!["push"];
    if let Some(ref r) = remote {
        args.push(r);
    }
    if let Some(ref b) = branch {
        args.push(b);
    }

    let output = std::process::Command::new("git")
        .args(&args)
        .current_dir(project_root)
        .output()
        .map_err(|e| format!("Failed to push: {}", e))?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Push failed: {}", error));
    }

    let result = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(result)
}

/// Pull changes from remote
#[tauri::command]
async fn git_pull(state: State<'_, AppState>) -> Result<String, String> {
    tracing::info!("git_pull command called");
    let project_root = state.project_root.lock().unwrap().clone();

    let output = std::process::Command::new("git")
        .args(&["pull"])
        .current_dir(project_root)
        .output()
        .map_err(|e| format!("Failed to pull: {}", e))?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Pull failed: {}", error));
    }

    let result = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(result)
}

/// Get current project root path
#[tauri::command]
async fn get_project_root(state: State<'_, AppState>) -> Result<String, String> {
    let project_root = state.project_root.lock().unwrap();
    Ok(project_root.to_string_lossy().to_string())
}

/// Open folder picker and set new project root
#[tauri::command]
async fn open_folder(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    use tauri::Emitter;
    use tauri_plugin_dialog::DialogExt;

    tracing::info!("open_folder command called");

    // Show folder picker dialog
    let folder_path = app
        .dialog()
        .file()
        .set_title("Select Project Folder")
        .blocking_pick_folder();

    if let Some(folder) = folder_path {
        let path = folder.into_path().map_err(|e| format!("Failed to convert path: {}", e))?;
        tracing::info!("Selected folder: {:?}", path);

        // Update project root
        {
            let mut project_root = state.project_root.lock().unwrap();
            *project_root = path.clone();
        }

        // Emit event to refresh frontend
        app.emit("project-folder-changed", path.to_string_lossy().to_string())
            .map_err(|e| format!("Failed to emit event: {}", e))?;

        Ok(path.to_string_lossy().to_string())
    } else {
        Err("No folder selected".to_string())
    }
}

// ============================================================================
// MAIN
// ============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    init_tracing();
    tracing::info!("Starting AuroraHeart IDE (Tauri)");

    // Get current directory
    let current_dir = std::env::current_dir()?;
    tracing::info!("Current directory: {:?}", current_dir);

    // Try to find project root
    let project_root = find_project_root(&current_dir).unwrap_or_else(|_| {
        tracing::warn!("Could not find project root, using current directory");
        current_dir.clone()
    });

    tracing::info!("Project root: {:?}", project_root);

    // Get project name
    let project_name = get_project_name(&project_root).unwrap_or_else(|| "Unknown".to_string());

    // Detect language
    let language = detect_language(&project_root).ok();
    if let Some(lang) = &language {
        tracing::info!("Detected language: {}", lang.as_str());
    }

    // Load configuration
    let config = match Config::load(&project_root) {
        Ok(config) => {
            tracing::info!("Loaded configuration");
            config
        }
        Err(ConfigError::ConfigDirNotFound(_)) => {
            tracing::info!("No configuration found, using defaults");
            let mut config = Config::default();
            config.project.root = Some(project_root.clone());
            config.project.name = Some(project_name.clone());
            config.project.language = language.map(|l| l.as_str().to_string());

            // Create .AuroraHeart directory
            let config_dir = project_root.join(".AuroraHeart");
            if !config_dir.exists() {
                std::fs::create_dir_all(&config_dir)?;
                tracing::info!("Created configuration directory");
            }

            // Save default configuration
            if let Err(e) = config.save(&project_root) {
                tracing::warn!("Failed to save configuration: {}", e);
            }

            config
        }
        Err(e) => {
            tracing::error!("Failed to load configuration: {}", e);
            return Err(e.into());
        }
    };

    tracing::debug!("Configuration: {:?}", config);

    // Create persistent conversation with system prompt
    let conversation = Arc::new(Mutex::new(Conversation::with_system_prompt(
        "You are Claude, a helpful AI assistant integrated into AuroraHeart IDE. \
         You help developers with coding tasks, explaining code, debugging, and general programming questions.",
    )));

    // Build and run Tauri application
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            // Create terminal manager with app handle
            let terminal_manager = TerminalManager::new(app.handle().clone());

            // Create application state
            let app_state = AppState {
                project_root: Arc::new(Mutex::new(project_root)),
                conversation,
                terminal_manager,
            };

            // Manage the state
            app.manage(app_state);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_file_tree,
            get_directory_contents,
            open_file,
            read_file_by_path,
            save_file,
            send_message,
            save_api_key,
            load_api_key,
            clear_chat,
            get_available_shells,
            get_default_shell,
            spawn_terminal,
            write_terminal,
            resize_terminal,
            close_terminal,
            list_terminals,
            get_git_status,
            is_file_modified,
            git_stage,
            git_unstage,
            git_commit,
            git_push,
            git_pull,
            get_project_root,
            open_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    tracing::info!("AuroraHeart IDE shutting down");
    Ok(())
}
