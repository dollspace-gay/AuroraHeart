//! AuroraHeart IDE - Main entry point
//!
//! This is the main executable for the AuroraHeart IDE, a Windows-native IDE
//! with first-class AI agent integration built in Rust with Slint.

use aurora_core::{
    detect_language, find_project_root, get_project_name, read_file, write_file, Config,
    ConfigError,
};
use std::path::Path;
use std::rc::Rc;

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    // Set up chat message callback
    let window_weak_chat = main_window.as_weak();
    main_window.on_send_message(move |message| {
        if let Some(window) = window_weak_chat.upgrade() {
            tracing::info!("Chat message: {}", message);

            let current_output = window.get_chat_output();
            let new_output = format!(
                "{}\n\n> {}\n\n[AI integration coming in Phase 2]\n",
                current_output, message
            );
            window.set_chat_output(new_output.into());
            window.set_status_message("Message sent (AI integration pending)".into());
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

    tracing::info!("Running main window");
    main_window.run()?;

    tracing::info!("AuroraHeart IDE shutting down");
    Ok(())
}
