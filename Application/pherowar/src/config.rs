use bincode_derive::{Decode, Encode};
use macroquad::prelude::Conf;
use serde::Deserialize;
use serde::Serialize;
use std::fs;
use std::path::Path;

// Window constants
pub const DEFAULT_WINDOW_WIDTH: f32 = 1920.0;
pub const DEFAULT_WINDOW_HEIGHT: f32 = 1080.0;

#[derive(Deserialize, Debug, Clone, Serialize, Encode, Decode)]
pub struct PlayerConfig {
    pub name: String,
    pub so_path: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SimulationConfig {
    pub colony_initial_population: u32,
    pub map: Option<String>,         // Optional map file to load at startup
    pub players_dir: Option<String>, // Directory for player .so files
    pub maps_dir: Option<String>,    // Directory for map files
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            colony_initial_population: 10000,
            map: None,
            players_dir: Some("players/".to_string()),
            maps_dir: Some("maps/".to_string()),
        }
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

pub fn load_player_configs(players_dir: Option<&str>) -> Vec<PlayerConfig> {
    let mut players = Vec::new();
    let dir = players_dir.unwrap_or("players/");
    let players_dir = Path::new(dir);
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
