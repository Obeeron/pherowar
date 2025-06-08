mod app;
mod config;
mod editor;
mod engine;
mod player;
mod simulation;
mod ui;

use std::path::PathBuf;

use app::PWApp;
use clap::Parser;
use config::{SimulationConfig, window_conf};
use toml;

/// Command-line arguments for PheroWar.
#[derive(Parser)]
#[command(name = "PheroWar", version, about = "PheroWar Simulation")]
pub struct Cli {
    /// Path to the TOML configuration file.
    #[arg(short, long, default_value = "./Application/config.toml")]
    config: Option<PathBuf>,

    /// Name of the map to load. Example: "Relic", "Labyrinth".
    #[arg(short, long)]
    map: Option<String>,

    /// List of colony players to spawn (player names separated by commas).
    #[arg(short = 'p', long, value_delimiter = ',')]
    players: Option<Vec<String>>,

    /// Evaluate mode: auto-start and exit when there is a winner. Requires players to be set and >= 2.
    #[arg(long)]
    evaluate: bool,
}

/// Loads the simulation configuration from a TOML file or uses defaults.
fn load_config(path: Option<PathBuf>) -> Result<SimulationConfig, Box<dyn std::error::Error>> {
    match path {
        Some(path) => {
            let content = match std::fs::read_to_string(&path) {
                Ok(content) => content,
                Err(e) => {
                    eprintln!("Failed to read config file '{}': {}", path.display(), e);
                    return Err(Box::new(e));
                }
            };

            let config = match toml::from_str(&content) {
                Ok(config) => config,
                Err(e) => {
                    eprintln!("Failed to parse config file: {}", e);
                    return Err(Box::new(e));
                }
            };
            println!("Loaded config from '{}'", path.display());
            println!("{:?}", config);
            Ok(config)
        }
        _ => {
            println!("No config file provided, using defaults.");
            Ok(SimulationConfig::default())
        }
    }
}

/// Main entry point for the PheroWar application.
#[macroquad::main(window_conf)]
async fn main() {
    let cli = Cli::parse();

    let config = match load_config(cli.config.clone()) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Error loading config: {}", e);
            return;
        }
    };

    // Create app config with validation
    let app_config = match config::AppConfig::from_cli_and_config(cli, config) {
        Ok(app_config) => app_config,
        Err(e) => {
            eprintln!("Error: {}", e);
            return;
        }
    };

    let mut app = match PWApp::new(app_config).await {
        Ok(app) => app,
        Err(e) => {
            eprintln!("Error creating application: {}", e);
            return;
        }
    };

    app.run().await;
}
