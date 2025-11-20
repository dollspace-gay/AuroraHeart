//! AuroraHeart IDE - Tauri Backend
//!
//! This is the Tauri backend for AuroraHeart IDE, providing commands for
//! file operations, AI agent integration, and credential management.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use aurora_agent::{AgenticEvent, AnthropicClient, Conversation, ToolExecutor};
use aurora_core::{
    detect_language, find_project_root, get_project_name, read_file, write_file, Config,
    ConfigError, CredentialStore,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::State;

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
    pub project_root: PathBuf,
    pub conversation: Arc<Mutex<Conversation>>,
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
    Ok(load_file_tree_internal(&state.project_root))
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

    let store = CredentialStore::for_project(&state.project_root);
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

    let store = CredentialStore::for_project(&state.project_root);
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
    let store = CredentialStore::for_project(&state.project_root);
    let api_key = store
        .retrieve("anthropic_api_key", "auroraheart")
        .map_err(|e| {
            let error_msg = "⚠ No API key configured. Please set your API key in Settings.";
            tracing::error!("Failed to load API key: {}", e);
            error_msg.to_string()
        })?;

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
    let executor = ToolExecutor::with_working_directory(state.project_root.clone());

    // Clone conversation for agentic loop
    let mut conv = state.conversation.lock().unwrap().clone();

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

    // Create application state
    let app_state = AppState {
        project_root,
        conversation,
    };

    // Build and run Tauri application
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(app_state)
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    tracing::info!("AuroraHeart IDE shutting down");
    Ok(())
}
