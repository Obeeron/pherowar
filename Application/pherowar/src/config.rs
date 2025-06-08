use bincode_derive::{Decode, Encode};
use macroquad::prelude::Conf;
use serde::Deserialize;
use serde::Serialize;
use std::fs;
use std::path::Path;

// Window constants
pub const DEFAULT_WINDOW_WIDTH: f32 = 1920.0;
pub const DEFAULT_WINDOW_HEIGHT: f32 = 1080.0;

// Directory path constants
pub const MAPS_DIR: &str = "./Application/maps/";
pub const PLAYERS_DIR: &str = "./players/";
pub const ASSETS_DIR: &str = "./Application/assets/";

#[derive(Deserialize, Debug, Clone, Serialize, Encode, Decode)]
pub struct PlayerConfig {
    pub name: String,
    pub so_path: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SimulationConfig {
    pub colony_initial_population: u32,
}

/// Configuration for the entire application including CLI parameters
pub struct AppConfig {
    pub simulation: SimulationConfig,
    pub cli_players: Option<Vec<String>>,
    pub player_configs: Vec<PlayerConfig>,
    pub map_name: Option<String>,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            colony_initial_population: 10000,
        }
    }
}

impl AppConfig {
    pub fn from_cli_and_config(
        cli: crate::Cli,
        simulation: SimulationConfig,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let cli_players = cli.players;
        let map_name = cli.map.or_else(|| Self::find_first_available_map());

        let player_configs = load_player_configs();

        if cli_players.is_some() && map_name.is_none() {
            return Err("CLI players provided but no map specified".into());
        }

        Ok(Self {
            simulation,
            cli_players,
            player_configs,
            map_name,
        })
    }

    /// Find the first available map in the maps directory
    fn find_first_available_map() -> Option<String> {
        let maps_dir = Path::new(MAPS_DIR);
        if let Ok(entries) = fs::read_dir(maps_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    if ext == "map" {
                        if let Some(file_name) = path.file_name() {
                            return Some(file_name.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }
        None
    }
}

pub fn window_conf() -> Conf {
    Conf {
        window_title: "PheroWar".to_owned(),
        window_width: DEFAULT_WINDOW_WIDTH as i32,
        window_height: DEFAULT_WINDOW_HEIGHT as i32,
        high_dpi: true,
        ..Default::default()
    }
}

pub fn load_player_configs() -> Vec<PlayerConfig> {
    let mut players = Vec::new();
    let players_dir = Path::new(PLAYERS_DIR);
    if let Ok(entries) = fs::read_dir(players_dir) {
        for entry in entries.flatten() {
            let path = entry.path().canonicalize().unwrap_or_default();
            if let Some(ext) = path.extension() {
                if ext == "so" {
                    if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                        players.push(PlayerConfig {
                            name: name.to_string(),
                            so_path: path.to_string_lossy().to_string(),
                        });
                    }
                }
            }
        }
    } else {
        eprintln!("Warning: players directory not found");
    }
    players.sort_by(|a, b| a.name.cmp(&b.name));
    players
}
