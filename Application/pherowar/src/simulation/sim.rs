use macroquad::prelude::*;
use macroquad::rand;
use std::collections::HashMap;

use crate::config::{PlayerConfig, SimulationConfig};

use super::ant::{Ant, AntRef};
use super::colony::Colony;
use super::map::GameMap;
use super::{DEFAULT_MAP_HEIGHT, DEFAULT_MAP_WIDTH, MAX_COLONIES, Terrain};

pub struct Simulation {
    pub tick: u32,
    pub map: GameMap,
    pub colonies: HashMap<u32, Colony>,
    pub player_configs: Vec<PlayerConfig>,
    pub is_paused: bool,
    pub config: SimulationConfig,
}

impl Simulation {
    pub fn new(
        config: &SimulationConfig,
        player_configs: Vec<PlayerConfig>,
        map_name: Option<String>,
    ) -> Self {
        let map = if let Some(ref name) = map_name {
            match GameMap::load_map_with_dir(name, config.maps_dir.as_deref()) {
                Ok(mut map) => {
                    map.loaded_map_name = Some(name.clone());
                    map
                }
                Err(e) => {
                    eprintln!("Failed to load map '{}': {}. Using empty map.", name, e);
                    GameMap::new(DEFAULT_MAP_WIDTH, DEFAULT_MAP_HEIGHT)
                }
            }
        } else {
            GameMap::new(DEFAULT_MAP_WIDTH, DEFAULT_MAP_HEIGHT)
        };

        Self {
            tick: 0,
            map,
            colonies: HashMap::with_capacity(MAX_COLONIES),
            player_configs,
            is_paused: true,
            config: config.clone(),
        }
    }

    pub fn update(&mut self, dt: f32) {
        if !self.is_paused {
            self.tick(dt);
            self.tick += 1;
        }
    }

    pub fn try_toggle_pause(&mut self) -> Result<(), String> {
        if self.is_paused {
            if !self.map.placeholder_colony_locations.is_empty() {
                return Err(
                    "Cannot unpause while placeholder colonies exist on the map.".to_string(),
                );
            }
            self.unpause();
        } else {
            self.pause();
        }
        Ok(())
    }

    pub fn tick(&mut self, dt: f32) {
        let mut colony_ids: Vec<u32> = self.colonies.keys().cloned().collect();
        // Shuffle colony processing order
        let n = colony_ids.len();
        for i in (1..n).rev() {
            let j = rand::gen_range(0, i + 1);
            colony_ids.swap(i, j);
        }

        for colony_id in &colony_ids {
            // Temporarily remove the current colony to pass the rest as &mut all_colonies
            if let Some(mut current_colony) = self.colonies.remove(colony_id) {
                current_colony.update(&mut self.map, &mut self.colonies, dt);
                // Put the colony back after its update
                self.colonies.insert(*colony_id, current_colony);
            }
        }
    }

    pub fn spawn_colony(&mut self, pos: Vec2, color: Color, player_cfg: PlayerConfig) {
        if self.colonies.len() >= MAX_COLONIES {
            eprintln!("Max colonies reached. Cannot spawn new colony.");
            return;
        }

        let mut colony_id: Option<u32> = None;
        for i in 0..MAX_COLONIES as u32 {
            if !self.colonies.contains_key(&i) {
                colony_id = Some(i);
                break;
            }
        }

        let current_colony_id = match colony_id {
            Some(id) => id,
            None => {
                eprintln!(
                    "No available colony ID found (this should not happen if MAX_COLONIES check passed)."
                );
                return;
            }
        };

        // Attempt to remove any placeholder status at this position first.
        // The position for remove_placeholder_colony should be the tile coordinates,
        // while 'pos' for spawn_colony is usually the center of the tile.
        let tile_pos = Vec2::new(pos.x.floor(), pos.y.floor());
        self.map.remove_placeholder_colony(tile_pos);

        match Colony::new(
            current_colony_id,
            pos,
            self.map.width,
            self.map.height,
            color,
            self.config.colony_initial_population,
            player_cfg.clone(),
        ) {
            Ok(mut new_colony) => {
                let x = pos.x.floor() as usize;
                let y = pos.y.floor() as usize;
                self.map.place_colony_at(x, y, current_colony_id);

                new_colony.spawn_ants(&mut self.map, self.config.colony_initial_population);
                self.colonies.insert(current_colony_id, new_colony);
            }
            Err(e) => {
                eprintln!("Failed to create colony: {}", e);
            }
        }
    }

    pub fn place_wall_at(&mut self, x: usize, y: usize) {
        let ants_to_respawn: Vec<AntRef> = self.map.ants_in_cell[y][x].iter().cloned().collect();

        if !(self.map.place_wall_at(x, y)) {
            return;
        }

        for ant_ref_to_respawn in ants_to_respawn {
            let colony_id = ant_ref_to_respawn.colony_id;
            if let Some(colony) = self.colonies.get_mut(&colony_id) {
                colony.respawn_ant(ant_ref_to_respawn.key, colony.pos, &mut self.map);
            } else {
                eprintln!(
                    "Warning: Colony {} for AntKey {:?} (from cell {},{} being walled) not found. Ant cannot be respawned.",
                    colony_id, ant_ref_to_respawn.key, x, y
                );
            }
        }
    }

    pub fn place_food_at(&mut self, x: usize, y: usize, amount: u32) {
        self.map.place_food_at(x, y, amount);
    }

    pub fn remove_terrain_at(&mut self, x: usize, y: usize) {
        self.map.remove_terrain_at(x, y);
    }

    pub fn get_terrain_at(&self, x: usize, y: usize) -> Option<&Terrain> {
        self.map.get_terrain_at(x, y)
    }

    pub fn get_ant(&self, ant_ref: &AntRef) -> Option<&Ant> {
        self.colonies
            .get(&ant_ref.colony_id)
            .and_then(|colony| colony.ants.get(ant_ref.key))
    }

    pub fn get_ant_at_world_pos(&self, world_pos: Vec2, click_radius: f32) -> Option<AntRef> {
        let cell_x = world_pos.x.floor() as isize;
        let cell_y = world_pos.y.floor() as isize;

        if cell_x < 0
            || cell_y < 0
            || cell_x >= self.map.width as isize
            || cell_y >= self.map.height as isize
        {
            return None;
        }

        let mut closest_ant: Option<AntRef> = None;
        let mut min_dist_sq = click_radius * click_radius;

        for dy in -1..=1 {
            for dx in -1..=1 {
                let check_x = cell_x + dx;
                let check_y = cell_y + dy;

                if check_x >= 0
                    && check_y >= 0
                    && check_x < self.map.width as isize
                    && check_y < self.map.height as isize
                {
                    let ants_in_cell = &self.map.ants_in_cell[check_y as usize][check_x as usize];
                    for ant_ref in ants_in_cell {
                        if let Some(ant) = self.get_ant(ant_ref) {
                            let dist_sq = ant.pos.distance_squared(world_pos);
                            if dist_sq < min_dist_sq {
                                min_dist_sq = dist_sq;
                                closest_ant = Some(ant_ref.clone());
                            }
                        }
                    }
                }
            }
        }
        closest_ant
    }

    pub fn place_nest_placeholder_at(&mut self, x: usize, y: usize) -> bool {
        if self.map.place_nest_placeholder_at(x, y) {
            self.pause(); // Pause the simulation when placing a placeholder
            return true;
        }
        return false;
    }

    pub fn remove_colony(&mut self, colony_id: u32) -> bool {
        if let Some(colony) = self.colonies.remove(&colony_id) {
            let x = colony.pos.x.floor() as usize;
            let y = colony.pos.y.floor() as usize;

            self.map.remove_colony_ants(colony_id);

            if let Some(Terrain::Nest(id)) = self.map.get_terrain_at(x, y) {
                if *id == colony_id {
                    self.map.remove_terrain_at(x, y);
                }
            }
            return true;
        }
        false // Colony not found
    }

    pub fn reset_colonies(&mut self) {
        let mut colony_spawn_data = Vec::new();
        for (_, colony) in &self.colonies {
            colony_spawn_data.push((colony.pos, colony.color, colony.player_config.clone()));
        }

        self.colonies.clear();

        self.map.soft_reset();

        for (pos, color, player_cfg) in colony_spawn_data.into_iter() {
            println!("Spawning colony at {:?} with color {:?}", pos, color);
            self.spawn_colony(pos, color, player_cfg);
        }
    }

    pub fn pause(&mut self) {
        self.is_paused = true;
    }

    pub fn unpause(&mut self) {
        self.is_paused = false;
    }

    pub fn reset(&mut self) {
        self.pause();
        self.tick = 0;

        if let Some(ref name) = self.map.loaded_map_name.clone() {
            match GameMap::load_map_with_dir(name, self.config.maps_dir.as_deref()) {
                Ok(mut loaded_map) => {
                    loaded_map.loaded_map_name = Some(name.clone());
                    self.map = loaded_map;

                    // Nests are managed by colonies, so remove any nest terrain from the loaded map
                    // as reset_colonies will handle spawning them.
                    for y in 0..self.map.height as usize {
                        for x in 0..self.map.width as usize {
                            if matches!(self.map.get_terrain_at(x, y), Some(Terrain::Nest(_))) {
                                self.map.remove_terrain_at(x, y);
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!(
                        "Failed to reload map '{}': {}. Only resetting colonies.",
                        name, e
                    );
                }
            }
        } else {
            // No map was loaded, so it's a new/default map. Only reset colonies and ant state
            self.map.soft_reset();
        }

        // Reset current configuration of simulation colonies
        self.reset_colonies();
    }

    pub fn create_new_map(&mut self, width: u32, height: u32) {
        self.map = GameMap::new(width, height);
        self.colonies.clear();
        self.tick = 0;
        self.pause();
    }

    /// Returns the total number of ants across all colonies
    pub fn total_ant_count(&self) -> usize {
        self.colonies.values().map(|colony| colony.ants.len()).sum()
    }
}
