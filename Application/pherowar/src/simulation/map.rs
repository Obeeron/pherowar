use crate::config::MAPS_DIR;
use crate::simulation::ant::AntRef;
use bincode::{decode_from_slice, encode_to_vec};
use bincode_derive::{Decode, Encode};
use macroquad::math::Vec2;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

use super::{DEFAULT_FOOD_AMOUNT, RaycastCache};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Encode, Decode)]
pub enum Terrain {
    Empty,
    Wall,
    Food(u32),
    Nest(u32),
    PlaceholderColony,
}

#[derive(Clone, Serialize, Deserialize, Encode, Decode)]
pub struct Tile {
    pub terrain: Terrain,
}

impl Default for Tile {
    fn default() -> Self {
        Self {
            terrain: Terrain::Empty,
        }
    }
}

pub struct GameMap {
    pub width: u32,
    pub height: u32,
    tiles: Vec<Vec<Tile>>,
    pub placeholder_colony_locations: Vec<Vec2>,
    pub ants_in_cell: Vec<Vec<HashSet<AntRef>>>,
    pub loaded_map_name: Option<String>,
    pub rc_cache: RaycastCache,
}

#[derive(Serialize, Deserialize, Clone, Encode, Decode)]
pub struct SerializedMap {
    pub width: u32,
    pub height: u32,
    pub tiles: Vec<Vec<Tile>>,
}

impl From<&GameMap> for SerializedMap {
    fn from(map: &GameMap) -> Self {
        let mut tiles = Vec::with_capacity(map.height as usize);
        for row_idx in 0..map.height as usize {
            let mut new_row = Vec::with_capacity(map.width as usize);
            for col_idx in 0..map.width as usize {
                let original_tile = &map.tiles[row_idx][col_idx];
                let new_terrain = match original_tile.terrain {
                    Terrain::Nest(_) => Terrain::PlaceholderColony,
                    Terrain::Food(_) => Terrain::Food(DEFAULT_FOOD_AMOUNT), // Reset food to default on save
                    _ => original_tile.terrain.clone(),
                };
                new_row.push(Tile {
                    terrain: new_terrain,
                });
            }
            tiles.push(new_row);
        }

        SerializedMap {
            width: map.width,
            height: map.height,
            tiles,
        }
    }
}

impl From<SerializedMap> for GameMap {
    fn from(smap: SerializedMap) -> Self {
        let mut game_map = GameMap::new(smap.width, smap.height);

        for (y, row) in smap.tiles.into_iter().enumerate() {
            for (x, tile_data) in row.into_iter().enumerate() {
                match tile_data.terrain {
                    Terrain::Nest(_) => {
                        eprintln!(
                            "Warning: Found Nest in loaded map data at ({}, {}), treating as Empty and placing placeholder.",
                            x, y
                        );
                        game_map.place_nest_placeholder_at(x, y);
                    }
                    Terrain::PlaceholderColony => {
                        game_map.place_nest_placeholder_at(x, y);
                    }
                    Terrain::Food(amount) => {
                        game_map.place_food_at(x, y, amount);
                    }
                    Terrain::Wall => {
                        game_map.tiles[y][x].terrain = Terrain::Wall;
                    }
                    Terrain::Empty => {}
                };
            }
        }

        game_map.rc_cache.clear();
        game_map.rc_cache.recompute_all_cache(&|gx, gy| {
            if gx < game_map.width as usize && gy < game_map.height as usize {
                matches!(game_map.tiles[gy][gx].terrain, Terrain::Wall)
            } else {
                true // Treat out-of-bounds as a wall for raycasting purposes
            }
        });

        game_map.loaded_map_name = None;
        game_map
    }
}

impl GameMap {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            tiles: vec![vec![Tile::default(); width as usize]; height as usize],
            placeholder_colony_locations: Vec::new(), // Initialize new field
            ants_in_cell: vec![vec![HashSet::new(); width as usize]; height as usize],
            loaded_map_name: None,
            rc_cache: RaycastCache::new(width as usize, height as usize),
        }
    }

    #[inline(always)]
    pub fn get_terrain_at(&self, x: usize, y: usize) -> Option<&Terrain> {
        if x < self.width as usize && y < self.height as usize {
            return Some(&self.tiles[y][x].terrain);
        }
        return None;
    }

    #[inline(always)]
    pub fn place_food_at(&mut self, x: usize, y: usize, amount: u32) {
        if x < self.width as usize && y < self.height as usize {
            self.tiles[y][x].terrain = Terrain::Food(amount);
        }
    }

    #[inline(always)]
    pub fn place_colony_at(&mut self, x: usize, y: usize, colony_id: u32) {
        if x < self.width as usize && y < self.height as usize {
            self.tiles[y][x].terrain = Terrain::Nest(colony_id);
        }
    }

    #[inline(always)]
    pub fn place_nest_placeholder_at(&mut self, x: usize, y: usize) -> bool {
        if x < self.width as usize && y < self.height as usize {
            if self.tiles[y][x].terrain == Terrain::Empty {
                self.tiles[y][x].terrain = Terrain::PlaceholderColony;
                let center_pos = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                if !self.placeholder_colony_locations.contains(&center_pos) {
                    self.placeholder_colony_locations.push(center_pos);
                    return true;
                }
            }
        }
        false
    }

    #[inline(always)]
    pub fn place_wall_at(&mut self, x: usize, y: usize) -> bool {
        if x < self.width as usize && y < self.height as usize {
            self.tiles[y][x].terrain = Terrain::Wall;
            self.rc_cache.invalidate_area_around(x, y);
            return true;
        }
        false
    }

    #[inline(always)]
    pub fn remove_terrain_at(&mut self, x: usize, y: usize) {
        if x < self.width as usize && y < self.height as usize {
            let was_wall = matches!(self.tiles[y][x].terrain, Terrain::Wall);
            self.tiles[y][x].terrain = Terrain::Empty;
            // If we removed a wall, invalidate raycast cache around this position
            if was_wall {
                self.rc_cache.invalidate_area_around(x, y);

                // This cell itself is no longer a wall, so its own outgoing rays need recomputation.
                let is_wall_check_fn = |gx: usize, gy: usize| {
                    if gx < self.width as usize && gy < self.height as usize {
                        matches!(self.tiles[gy][gx].terrain, Terrain::Wall)
                    } else {
                        true
                    }
                };
                self.rc_cache
                    .recompute_all_rays_for_cell(&is_wall_check_fn, x, y);
            }
        }
    }

    pub fn remove_placeholder_colony(&mut self, pos: Vec2) -> bool {
        let ix = pos.x as i32;
        let iy = pos.y as i32;
        let mut cleared_tile = false;
        let mut removed_from_list = false;

        if ix >= 0 && ix < self.width as i32 && iy >= 0 && iy < self.height as i32 {
            let ux = ix as usize;
            let uy = iy as usize;
            if self.get_terrain_at(ux, uy) == Some(&Terrain::PlaceholderColony) {
                self.remove_terrain_at(ux, uy);
                cleared_tile = true;
            }

            // Always attempt to remove from the list for consistency
            let center_pos = Vec2::new(ix as f32 + 0.5, iy as f32 + 0.5);
            let initial_len = self.placeholder_colony_locations.len();
            self.placeholder_colony_locations
                .retain(|&p| p != center_pos);
            removed_from_list = self.placeholder_colony_locations.len() < initial_len;
        }
        cleared_tile || removed_from_list
    }

    /// Registers an ant in the spatial grid for a specific cell.
    pub fn register_ant_in_cell(&mut self, ant_ref: &AntRef, pos: Vec2) {
        let cell_x = pos.x.floor() as isize;
        let cell_y = pos.y.floor() as isize;

        if cell_x >= 0
            && cell_y >= 0
            && (cell_x as usize) < self.width as usize
            && (cell_y as usize) < self.height as usize
        {
            self.ants_in_cell[cell_y as usize][cell_x as usize].insert(ant_ref.clone());
        } else {
            eprintln!(
                "Warning: Ant {:?} attempted to register at out-of-bounds pos ({:.2},{:.2}). Not registered.",
                ant_ref, pos.x, pos.y
            );
        }
    }

    /// Unregisters an ant from the spatial grid for a specific cell.
    /// Returns true if the ant was found in the specified cell and removed, false otherwise.
    pub fn unregister_ant_from_cell(&mut self, ant_ref: &AntRef, pos: Vec2) -> bool {
        let cell_x = pos.x.floor() as isize;
        let cell_y = pos.y.floor() as isize;

        if cell_x >= 0
            && cell_y >= 0
            && (cell_x as usize) < self.width as usize
            && (cell_y as usize) < self.height as usize
        {
            return self.ants_in_cell[cell_y as usize][cell_x as usize].remove(ant_ref);
        }
        eprintln!(
            "Warning: Ant {:?} attempted to unregister from out-of-bounds pos ({:.2},{:.2}). Not unregistered.",
            ant_ref, pos.x, pos.y
        );
        false
    }

    /// Save the map
    pub fn save_map<P: AsRef<Path>>(&mut self, name: P) -> io::Result<()> {
        let dir = std::path::Path::new(MAPS_DIR);
        if !dir.exists() {
            fs::create_dir_all(&dir)?;
        }
        let file_path = dir.join(name.as_ref());
        let serialized = SerializedMap::from(&*self); // Changed to pass an immutable reference
        let data = encode_to_vec(&serialized, bincode::config::standard())
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        let mut file = fs::File::create(file_path)?;
        file.write_all(&data)?;
        self.loaded_map_name = Some(name.as_ref().to_string_lossy().to_string());
        Ok(())
    }

    /// Load a map and return a GameMap with loaded_map_name set.
    pub fn load_map<P: AsRef<Path>>(name: P) -> io::Result<GameMap> {
        let name_str = name.as_ref().to_string_lossy().to_string();
        let file_path = std::path::Path::new(MAPS_DIR).join(&name_str);
        let data = fs::read(file_path)?;
        let (serialized, _len): (SerializedMap, _) =
            decode_from_slice(&data, bincode::config::standard())
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        let mut map: GameMap = serialized.into();
        println!("Loaded map {}", name_str);
        map.loaded_map_name = Some(name_str);
        Ok(map)
    }

    /// List all map files in the maps/ directory
    pub fn list_maps() -> io::Result<Vec<String>> {
        let maps_dir_path = std::path::Path::new(MAPS_DIR);
        if !maps_dir_path.exists() {
            return Ok(vec![]);
        }
        let mut maps = vec![];
        for entry in fs::read_dir(maps_dir_path)? {
            let entry = entry?;
            if let Some(name) = entry.file_name().to_str() {
                maps.push(name.to_string());
            }
        }
        Ok(maps)
    }

    pub fn take_food_at(&mut self, x: usize, y: usize) {
        if x < self.width as usize && y < self.height as usize {
            if let Terrain::Food(current_food) = &mut self.tiles[y][x].terrain {
                if *current_food >= 1 {
                    *current_food -= 1;
                    if *current_food == 0 {
                        self.tiles[y][x].terrain = Terrain::Empty;
                    }
                } else {
                    // Food amount was already 0 or less, ensure it's empty
                    self.tiles[y][x].terrain = Terrain::Empty;
                }
            }
        }
    }

    /// Get enemy ant at the given coordinates (x, y) that is not from the given colony
    pub fn get_enemy_ant_at(&self, x: usize, y: usize, friendly_colony_id: u32) -> Option<AntRef> {
        if let Some(ants_set) = self.ants_in_cell.get(y).and_then(|row| row.get(x)) {
            for ant_ref in ants_set {
                if ant_ref.colony_id != friendly_colony_id {
                    return Some(ant_ref.clone());
                }
            }
        }
        None
    }

    /// Only reset the ants data
    pub fn soft_reset(&mut self) {
        self.ants_in_cell
            .iter_mut()
            .for_each(|row| row.iter_mut().for_each(|cell_set| cell_set.clear()));
    }

    /// Remove a colony from the map
    /// This will remove all ants from the map and clear the pheromone channels.
    pub fn remove_colony_ants(&mut self, colony_id: u32) {
        // Clear all ants of this colony from the map
        for y in 0..self.height as usize {
            for x in 0..self.width as usize {
                self.ants_in_cell[y][x].retain(|ant_ref| ant_ref.colony_id != colony_id);
            }
        }
    }

    /// Perform a raycast from the given position at the given angle.
    /// The ray is traced up to `SENSE_MAX_DISTANCE` by the underlying cache.
    /// This function then interprets the result based on the provided `max_distance_for_query`.
    ///
    /// Returns:
    ///  - `(true, distance_to_wall)`: If a wall is hit within `max_distance_for_query`.
    ///  - `(false, max_distance_for_query)`: If no wall is hit within `max_distance_for_query`.
    ///  - `(true, 0.0)`: If the `start_pos` is outside map bounds or inside a wall.
    pub fn raycast_angle(
        &mut self,
        start_pos: Vec2,
        angle: f32,
        max_distance_for_query: f32,
    ) -> (bool, f32) {
        let grid_x = start_pos.x.floor() as usize;
        let grid_y = start_pos.y.floor() as usize;

        // Define the is_wall_fn closure based on the current map state.
        // This is used both for an early exit check and for the cache query.
        let is_wall_fn = |gx: usize, gy: usize| {
            if gx < self.width as usize && gy < self.height as usize {
                matches!(self.tiles[gy][gx].terrain, Terrain::Wall)
            } else {
                true // Treat out-of-bounds as a wall for raycasting purposes.
            }
        };

        // Early exit if starting position is outside map bounds (for cache access) or inside a wall.
        if grid_x >= self.width as usize
            || grid_y >= self.height as usize
            || is_wall_fn(grid_x, grid_y)
        {
            return (true, 0.0); // Blocked, zero distance.
        }

        match self
            .rc_cache
            .get_distance_at_angle(&is_wall_fn, grid_x, grid_y, angle)
        {
            Some(cached_distance_to_obstacle) => {
                // cached_distance_to_obstacle is the distance to a wall if found by cache (up to SENSE_MAX_DISTANCE),
                // or f32::INFINITY if no wall was hit by the cache within its sensing range.

                if cached_distance_to_obstacle < max_distance_for_query {
                    // A wall was hit by the cache, and it's closer than the query's specific max distance.
                    (true, cached_distance_to_obstacle)
                } else {
                    // No wall was hit by the cache within the query's specific max distance.
                    // This includes cases where:
                    //   1. Cache hit a wall, but it's >= max_distance_for_query.
                    //   2. Cache hit no wall at all (cached_distance_to_obstacle is INFINITY).
                    (false, max_distance_for_query)
                }
            }
            None => {
                // This case implies the (grid_x, grid_y) was outside the cache's dimensions,
                // which should have been caught by the initial boundary check.
                // If it occurs, treat as an error/unexpected state.
                eprintln!(
                    "Warning: RaycastCache returned None for an apparently in-bounds origin ({}, {}). This indicates a potential issue.",
                    grid_x, grid_y
                );
                (true, 0.0) // Default to blocked at origin for safety.
            }
        }
    }
}
