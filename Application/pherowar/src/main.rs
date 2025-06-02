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
struct Cli {
    /// Path to the TOML configuration file.
    #[arg(short, long)]
    config: Option<PathBuf>,
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
            println!("Config: {:?}", config);
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

    match load_config(cli.config) {
        Ok(config) => {
            let players = config::load_player_configs(config.players_dir.as_deref());
            let mut app = PWApp::new(config, players).await;
            app.run().await;
        }
        Err(e) => {
            eprintln!("Error loading config: {}", e);
        }
    }
}
