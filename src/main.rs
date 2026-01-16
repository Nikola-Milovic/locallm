mod clipboard;
mod config;
mod gpu_stats;
mod ollama;
mod ui;

use config::Config;
use iced::window;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn main() -> iced::Result {
    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "locallm=info".into()),
        )
        .init();

    tracing::info!("Starting LocalLM");

    // Load configuration
    let config = match Config::load() {
        Ok(cfg) => cfg,
        Err(e) => {
            tracing::warn!("Failed to load config, using defaults: {e}");
            Config::default()
        }
    };

    tracing::info!("Ollama URL: {}", config.ollama_url);
    tracing::info!("Config path: {:?}", Config::config_path());

    // Create and run the application
    iced::application(ui::App::title, ui::App::update, ui::App::view)
        .subscription(ui::App::subscription)
        .theme(ui::App::theme)
        .window(window::Settings {
            size: iced::Size::new(800.0, 700.0),
            min_size: Some(iced::Size::new(500.0, 400.0)),
            resizable: true,
            platform_specific: window::settings::PlatformSpecific {
                application_id: String::from("locallm"),
                ..Default::default()
            },
            ..Default::default()
        })
        .run_with(|| ui::App::new(config))
}
