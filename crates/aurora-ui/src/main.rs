//! AuroraHeart IDE - Main entry point
//!
//! This is the main executable for the AuroraHeart IDE, a Windows-native IDE
//! with first-class AI agent integration built in Rust with Slint.

use aurora_core::{Config, ConfigError};

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    init_tracing();
    tracing::info!("Starting AuroraHeart IDE");

    // Load configuration
    let current_dir = std::env::current_dir()?;
    let config = match Config::load(&current_dir) {
        Ok(config) => {
            tracing::info!("Loaded configuration from {:?}", current_dir);
            config
        }
        Err(ConfigError::ConfigDirNotFound(_)) => {
            tracing::info!("No configuration found, using defaults");
            let mut config = Config::default();
            config.project.root = Some(current_dir.clone());

            // Create .AuroraHeart directory if it doesn't exist
            let config_dir = current_dir.join(".AuroraHeart");
            if !config_dir.exists() {
                std::fs::create_dir_all(&config_dir)?;
                tracing::info!("Created configuration directory: {:?}", config_dir);
            }

            // Save default configuration
            config.save(&current_dir)?;
            tracing::info!("Saved default configuration");

            config
        }
        Err(e) => {
            tracing::error!("Failed to load configuration: {}", e);
            return Err(e.into());
        }
    };

    tracing::debug!("Configuration: {:?}", config);

    // Create and run the main window
    let main_window = MainWindow::new()?;

    // Set initial state
    main_window.set_editor_text("// Welcome to AuroraHeart IDE\n// Your AI-powered Rust development environment\n".into());
    main_window.set_chat_output("Hello! I'm Claude, your AI coding assistant.\nHow can I help you today?".into());

    // Set up callbacks
    main_window.on_open_file(move || {
        tracing::info!("Open file requested");
        // TODO: Implement file opening dialog
    });

    main_window.on_save_file(move || {
        tracing::info!("Save file requested");
        // TODO: Implement file saving
    });

    let window_weak = main_window.as_weak();
    main_window.on_send_message(move |message| {
        tracing::info!("Message sent: {}", message);
        // TODO: Implement AI agent integration
        if let Some(window) = window_weak.upgrade() {
            let current_output = window.get_chat_output();
            let new_output = format!("{}\n\nYou: {}\n\nClaude: [AI integration coming in Phase 2]",
                                      current_output, message);
            window.set_chat_output(new_output.into());
        }
    });

    tracing::info!("Running main window");
    main_window.run()?;

    tracing::info!("AuroraHeart IDE shutting down");
    Ok(())
}
