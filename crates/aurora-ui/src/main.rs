//! AuroraHeart IDE - Main entry point
//!
//! This is the main executable for the AuroraHeart IDE, a Windows-native IDE
//! with first-class AI agent integration built in Rust with Slint.

use aurora_agent::{AnthropicClient, Conversation, StreamEvent};
use aurora_core::{
    detect_language, find_project_root, get_project_name, read_file, write_file, Config,
    ConfigError, CredentialStore,
};
use futures::StreamExt;
use std::path::Path;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

slint::include_modules!();

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

/// Load files from current directory into file tree
fn load_file_tree<P: AsRef<Path>>(dir: P) -> Vec<FileTreeItem> {
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
                name: file_name.into(),
                path: path_str.into(),
                is_directory,
            });
        }
    }

    // Sort: directories first, then files, both alphabetically
    items.sort_by(|a, b| {
        match (a.is_directory, b.is_directory) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });

    items
}

/// Open and display a file in the editor
fn open_file_in_editor(main_window: &MainWindow, file_path: &str) {
    match read_file(file_path) {
        Ok(content) => {
            main_window.set_editor_text(content.clone().into());
            main_window.set_original_content(content.into());
            main_window.set_current_file(file_path.into());
            main_window.set_is_modified(false);

            let file_name = Path::new(file_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(file_path);

            main_window.set_status_message(format!("Opened: {}", file_name).into());
            tracing::info!("Opened file: {}", file_path);
        }
        Err(e) => {
            let error_msg = format!("Error opening file: {}", e);
            main_window.set_status_message(error_msg.clone().into());
            tracing::error!("{}", error_msg);
        }
    }
}

/// Save the current file
fn save_current_file(main_window: &MainWindow) {
    let current_file = main_window.get_current_file();
    let content = main_window.get_editor_text();

    if current_file.is_empty() {
        main_window.set_status_message("No file to save".into());
        return;
    }

    match write_file(current_file.as_str(), content.as_str()) {
        Ok(()) => {
            // Update original content and clear modified state
            main_window.set_original_content(content.clone());
            main_window.set_is_modified(false);

            let file_name = Path::new(current_file.as_str())
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("file");

            main_window.set_status_message(format!("Saved: {}", file_name).into());
            tracing::info!("Saved file: {}", current_file);
        }
        Err(e) => {
            let error_msg = format!("Error saving file: {}", e);
            main_window.set_status_message(error_msg.clone().into());
            tracing::error!("{}", error_msg);
        }
    }
}

/// Save API key to encrypted credential store
fn save_api_key(project_root: &Path, api_key: &str) -> Result<(), Box<dyn std::error::Error>> {
    let store = CredentialStore::for_project(project_root);
    store.store("anthropic_api_key", api_key, "auroraheart")?;
    tracing::info!("API key saved successfully");
    Ok(())
}

/// Load API key from encrypted credential store
fn load_api_key(project_root: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let store = CredentialStore::for_project(project_root);
    let api_key = store.retrieve("anthropic_api_key", "auroraheart")?;
    Ok(api_key)
}

/// Mask API key for display (show first 7 chars and last 4 chars)
fn mask_api_key(api_key: &str) -> String {
    if api_key.len() <= 11 {
        return "****".to_string();
    }

    let prefix = &api_key[..7];  // "sk-ant-"
    let suffix = &api_key[api_key.len()-4..];
    format!("{}...{}", prefix, suffix)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    init_tracing();
    tracing::info!("Starting AuroraHeart IDE");

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

    // Create and configure the main window
    let main_window = MainWindow::new()?;

    // Set initial state
    main_window.set_project_name(project_name.clone().into());
    main_window.set_status_message("Ready".into());

    // Load file tree
    let file_tree = load_file_tree(&project_root);
    let file_tree_model = Rc::new(slint::VecModel::from(file_tree));
    main_window.set_file_tree_items(file_tree_model.into());

    // Load and set masked API key if it exists
    if let Ok(api_key) = load_api_key(&project_root) {
        let masked = mask_api_key(&api_key);
        main_window.set_api_key_masked(masked.into());
        tracing::info!("Loaded existing API key");
    }

    // Set welcome message in chat
    let welcome_message = format!(
        "Hello! I'm Claude, your AI coding assistant.\n\nProject: {}\n{}Ready to help you code!",
        project_name,
        if let Some(lang) = language {
            format!("Language: {}\n", lang.as_str())
        } else {
            String::new()
        }
    );
    main_window.set_chat_output(welcome_message.into());

    // Set up file selection callback
    let window_weak_file = main_window.as_weak();
    main_window.on_file_selected(move |file_path| {
        if let Some(window) = window_weak_file.upgrade() {
            open_file_in_editor(&window, file_path.as_str());
        }
    });

    // Set up open file callback
    let window_weak_open = main_window.as_weak();
    main_window.on_open_file(move || {
        if let Some(window) = window_weak_open.upgrade() {
            tracing::info!("Open file dialog requested");
            window.set_status_message("File dialog not yet implemented".into());
            // TODO: Implement native file dialog in Phase 1 completion
        }
    });

    // Set up save file callback
    let window_weak_save = main_window.as_weak();
    main_window.on_save_file(move || {
        if let Some(window) = window_weak_save.upgrade() {
            save_current_file(&window);
        }
    });

    // Create persistent conversation with system prompt
    let conversation = Arc::new(Mutex::new(
        Conversation::with_system_prompt(
            "You are Claude, a helpful AI assistant integrated into AuroraHeart IDE. \
             You help developers with coding tasks, explaining code, debugging, and general programming questions."
        )
    ));

    // Set up chat message callback
    let window_weak_chat = main_window.as_weak();
    let project_root_chat = project_root.clone();
    let conversation_clone = conversation.clone();
    main_window.on_send_message(move |message| {
        if let Some(window) = window_weak_chat.upgrade() {
            tracing::info!("Chat message: {}", message);

            // Update UI to show user message
            let current_output = window.get_chat_output();
            let new_output = format!(
                "{}\n\n> You: {}\n\nClaude: ",
                current_output, message
            );
            window.set_chat_output(new_output.clone().into());
            window.set_status_message("Sending message to Claude...".into());

            // Load API key
            let api_key = match load_api_key(&project_root_chat) {
                Ok(key) => key,
                Err(e) => {
                    let error_msg = format!("{}⚠ No API key configured. Please set your API key in Settings.", new_output);
                    window.set_chat_output(error_msg.into());
                    window.set_status_message("API key not found".into());
                    tracing::error!("Failed to load API key: {}", e);
                    return;
                }
            };

            // Add user message to conversation and truncate if needed
            {
                let mut conv = conversation_clone.lock().unwrap();
                conv.add_user_message(message.as_str());
                // Keep conversation within reasonable limits (50k tokens = ~200k chars)
                let removed = conv.truncate_to_tokens(50_000);
                if removed > 0 {
                    tracing::info!("Truncated {} old messages from conversation", removed);
                }
            }

            // Create client and clone conversation for API call
            let client = AnthropicClient::new(api_key);
            let conversation_for_api = conversation_clone.lock().unwrap().clone();

            // Clone window weak and conversation for async task
            let window_weak_async = window_weak_chat.clone();
            let conversation_async = conversation_clone.clone();

            // Spawn async task to send message and stream response
            tokio::spawn(async move {
                match client.send_conversation_stream(&conversation_for_api).await {
                    Ok(mut stream) => {
                        let mut accumulated_text = String::new();

                        while let Some(event_result) = stream.next().await {
                            match event_result {
                                Ok(StreamEvent::ContentBlockDelta { delta, .. }) => {
                                    let aurora_agent::Delta::TextDelta { text } = delta;
                                    accumulated_text.push_str(&text);

                                    // Update UI with accumulated text
                                    let text_clone = accumulated_text.clone();
                                    let output_base = new_output.clone();
                                    let window_weak_clone = window_weak_async.clone();
                                    let _ = slint::invoke_from_event_loop(move || {
                                        if let Some(w) = window_weak_clone.upgrade() {
                                            let display_text = format!("{}{}", output_base, text_clone);
                                            w.set_chat_output(display_text.into());
                                        }
                                    });
                                }
                                Ok(StreamEvent::MessageStop) => {
                                    tracing::info!("Stream completed");

                                    // Save assistant's response to conversation
                                    if !accumulated_text.is_empty() {
                                        if let Ok(mut conv) = conversation_async.lock() {
                                            conv.add_assistant_message(&accumulated_text);
                                            tracing::debug!("Added assistant message to conversation");
                                        }
                                    }

                                    let window_weak_clone = window_weak_async.clone();
                                    let _ = slint::invoke_from_event_loop(move || {
                                        if let Some(w) = window_weak_clone.upgrade() {
                                            w.set_status_message("Message complete".into());
                                        }
                                    });
                                    break;
                                }
                                Ok(StreamEvent::Error { error }) => {
                                    tracing::error!("Stream error: {:?}", error);
                                    let error_msg = format!("\n\n⚠ Error: {}", error.message);
                                    let output_base = new_output.clone();
                                    let accumulated = accumulated_text.clone();
                                    let window_weak_clone = window_weak_async.clone();
                                    let _ = slint::invoke_from_event_loop(move || {
                                        if let Some(w) = window_weak_clone.upgrade() {
                                            let display = format!("{}{}{}", output_base, accumulated, error_msg);
                                            w.set_chat_output(display.into());
                                            w.set_status_message("Error occurred".into());
                                        }
                                    });
                                    break;
                                }
                                Ok(_) => {
                                    // Other events like Ping, MessageStart, etc.
                                }
                                Err(e) => {
                                    tracing::error!("Stream error: {:?}", e);
                                    let error_msg = format!("\n\n⚠ Error: {}", e);
                                    let output_base = new_output.clone();
                                    let accumulated = accumulated_text.clone();
                                    let window_weak_clone = window_weak_async.clone();
                                    let _ = slint::invoke_from_event_loop(move || {
                                        if let Some(w) = window_weak_clone.upgrade() {
                                            let display = format!("{}{}{}", output_base, accumulated, error_msg);
                                            w.set_chat_output(display.into());
                                            w.set_status_message("Connection error".into());
                                        }
                                    });
                                    break;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to send message: {:?}", e);
                        let error_msg = format!("\n\n⚠ Failed to send message: {}", e);
                        let output_base = new_output.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(w) = window_weak_async.upgrade() {
                                let display = format!("{}{}", output_base, error_msg);
                                w.set_chat_output(display.into());
                                w.set_status_message("Failed to connect".into());
                            }
                        });
                    }
                }
            });
        }
    });

    // Set up text editing callback to detect changes
    let window_weak_edit = main_window.as_weak();
    main_window.on_text_edited(move || {
        if let Some(window) = window_weak_edit.upgrade() {
            let current_text = window.get_editor_text();
            let original_text = window.get_original_content();
            let is_modified = current_text != original_text;
            window.set_is_modified(is_modified);
        }
    });

    // Set up save API key callback
    let project_root_save = project_root.clone();
    let window_weak_save_key = main_window.as_weak();
    main_window.on_save_api_key(move |api_key| {
        if let Some(window) = window_weak_save_key.upgrade() {
            match save_api_key(&project_root_save, api_key.as_str()) {
                Ok(()) => {
                    let masked = mask_api_key(api_key.as_str());
                    window.set_api_key_masked(masked.into());
                    window.set_status_message("API key saved successfully".into());
                    tracing::info!("API key saved via UI");
                }
                Err(e) => {
                    let error_msg = format!("Failed to save API key: {}", e);
                    window.set_status_message(error_msg.clone().into());
                    tracing::error!("{}", error_msg);
                }
            }
        }
    });

    // Set up load API key callback
    let project_root_load = project_root.clone();
    main_window.on_load_api_key(move || {
        match load_api_key(&project_root_load) {
            Ok(api_key) => {
                tracing::info!("API key loaded via UI");
                api_key.into()
            }
            Err(_) => {
                tracing::debug!("No API key found");
                "".into()
            }
        }
    });

    // Set up clear chat callback
    let window_weak_clear = main_window.as_weak();
    let conversation_clear = conversation.clone();
    main_window.on_clear_chat(move || {
        if let Some(window) = window_weak_clear.upgrade() {
            // Clear the conversation state
            if let Ok(mut conv) = conversation_clear.lock() {
                conv.clear();
                tracing::info!("Conversation cleared");
            }

            // Reset the chat UI with welcome message
            let welcome_message = format!(
                "Hello! I'm Claude, your AI coding assistant.\n\nConversation cleared. Ready to help you code!"
            );
            window.set_chat_output(welcome_message.into());
            window.set_status_message("Chat cleared".into());
        }
    });

    tracing::info!("Running main window");
    main_window.run()?;

    tracing::info!("AuroraHeart IDE shutting down");
    Ok(())
}
