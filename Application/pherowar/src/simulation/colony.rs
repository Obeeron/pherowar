use super::ant::{Ant, AntKey};
use super::pheromone::PheromoneChannel;
use super::{PHEROMONE_DECAY_INTERVAL, Timer};
use crate::config::PlayerConfig;
use crate::player::PlayerConnection;
use crate::simulation::Terrain;
use crate::simulation::{ANT_SPAWN_INTERVAL, GameMap};
use anyhow::Result;
use macroquad::prelude::*;
use shared::PHEROMONE_CHANNEL_COUNT;
use slotmap::SlotMap;
use std::collections::HashMap;

pub struct Colony {
    pub colony_id: u32,
    pub ants: SlotMap<AntKey, Ant>,
    pub pheromones: Vec<PheromoneChannel>,
    pub color: Color,
    pub pos: Vec2,
    pub food_collected: u32,
    pub player_connection: PlayerConnection,
    pub player_config: PlayerConfig,
    pub pheromone_decay_timer: Timer,
    pub ant_spawn_timer: f32,
}

impl Colony {
    pub fn new(
        colony_id: u32,
        pos: Vec2,
        map_width: u32,
        map_height: u32,
        color: Color,
        ant_count: u32,
        player_cfg: PlayerConfig,
    ) -> Result<Self> {
        let ants = SlotMap::with_capacity_and_key(ant_count as usize);

        // Start player connection and get decay rates from setup
        let player_connection = PlayerConnection::start(colony_id, &player_cfg)?;
        let decay_rates = player_connection.setup.decay_rates;
        let mut pheromones = Vec::with_capacity(PHEROMONE_CHANNEL_COUNT);
        for i in 0..PHEROMONE_CHANNEL_COUNT {
            pheromones.push(PheromoneChannel::new(map_width, map_height, decay_rates[i]));
        }

        // Check for all channels to make sure they are initialized correctly with 0.0 on all cells
        for (i, channel) in pheromones.iter().enumerate() {
            if channel
                .data
                .iter()
                .any(|row| row.iter().any(|&val| val != 0.0))
            {
                eprintln!(
                    "Warning: Pheromone channel {} initialized with non-zero values.",
                    i
                );
            }
        }

        Ok(Self {
            pos,
            ants,
            color,
            food_collected: 0,
            pheromones,
            colony_id,
            player_connection,
            player_config: player_cfg,
            pheromone_decay_timer: Timer::new(PHEROMONE_DECAY_INTERVAL, 0.0),
            ant_spawn_timer: 0.0,
        })
    }

    pub fn update(
        &mut self,
        map: &mut GameMap,
        other_colonies: &mut HashMap<u32, Colony>,
        dt: f32,
    ) {
        self.pheromone_decay_timer.update(dt);
        if self.pheromone_decay_timer.is_ready() {
            self.decay_pheromones();
            self.pheromone_decay_timer.wrap();
        }

        let (pheromones, player_connection, pos) =
            (&mut self.pheromones, &mut self.player_connection, self.pos);

        let mut ants_to_despawn: Vec<AntKey> = Vec::new();

        for (key, ant) in self.ants.iter_mut() {
            // Lose longevity (aging)
            ant.longevity -= dt; // longevity decreases
            if ant.longevity < 0.0 {
                ant.longevity = 0.0;
            }

            // Stop if dead (could be due to age or killed by enemy during the same tick)
            if ant.is_dead() {
                ants_to_despawn.push(key);
                continue;
            }

            // Update is_on_colony status
            ant.check_colony(&self.pos);
            // Update is_on_food status
            ant.check_food(map);

            // Try drop food on colony
            if ant.is_on_colony && ant.carrying_food {
                ant.carrying_food = false;
                self.food_collected += 1;
                ant.rejuvenate();
            }

            // Updates the ant's position, pheromone laying, and fighting logic
            ant.update(&pos, map, pheromones, player_connection, other_colonies, dt);
        }

        for key in ants_to_despawn {
            self.despawn_ant(key, map);
        }

        self.ant_spawn_timer += dt;
        while self.ant_spawn_timer >= ANT_SPAWN_INTERVAL
            && self.food_collected >= crate::simulation::ANT_SPAWN_FOOD_COST
        {
            self.spawn_ant(map);
            self.food_collected -= crate::simulation::ANT_SPAWN_FOOD_COST;
            self.ant_spawn_timer -= ANT_SPAWN_INTERVAL;
        }
    }

    fn decay_pheromones(&mut self) {
        for pheromone in &mut self.pheromones {
            pheromone.decay();
        }
    }

    pub fn spawn_ants(&mut self, map: &mut GameMap, count: u32) {
        for _ in 0..count {
            self.spawn_ant(map);
        }
    }

    pub fn spawn_ant(&mut self, map: &mut GameMap) {
        let mut ant_instance = Ant::new(self.pos, self.colony_id);
        let key = self.ants.insert_with_key(|k| {
            ant_instance.ant_ref.key = k;
            ant_instance
        });

        // Register the newly spawned ant in the map at its initial position.
        if let Some(new_ant) = self.ants.get(key) {
            map.register_ant_in_cell(&new_ant.ant_ref, new_ant.pos);
        } else {
            // This should not happen if insert_with_key succeeded.
            eprintln!(
                "Critical Error: AntKey {:?} not found immediately after insertion in spawn_ant for colony {}. Map registration skipped.",
                key, self.colony_id
            );
        }
    }

    pub fn despawn_ant(&mut self, key: AntKey, map: &mut GameMap) {
        if let Some(ant_to_despawn) = self.ants.get(key) {
            let ant_ref_clone = ant_to_despawn.ant_ref.clone();
            let ant_pos = ant_to_despawn.pos;
            // If the ant was carrying food, drop it on the terrain
            if ant_to_despawn.carrying_food {
                let x = ant_pos.x.floor() as usize;
                let y = ant_pos.y.floor() as usize;
                // Only place food if the cell is empty or already food
                if let Some(terrain) = map.get_terrain_at(x, y) {
                    match terrain {
                        Terrain::Empty => map.place_food_at(x, y, 1),
                        Terrain::Food(amount) => map.place_food_at(x, y, amount + 1),
                        _ => {}
                    }
                }
            }
            // Unregister the ant from the map at its last known position.
            if !map.unregister_ant_from_cell(&ant_ref_clone, ant_pos) {
                eprintln!(
                    "Warning: Ant {:?} (key {:?}) at pos ({:.2},{:.2}) was not found in its cell during despawn. It might have been already unregistered or desynced.",
                    ant_ref_clone, key, ant_pos.x, ant_pos.y
                );
            }

            // Now remove from the colony's own list.
            self.ants.remove(key);
        } else {
            eprintln!(
                "Warning: AntKey {:?} not found in colony {} ant list during despawn attempt.",
                key, self.colony_id
            );
        }
    }

    /// Respawns an ant by despawning the old one and spawning a new one at the colony's nest.
    /// The `new_pos` argument is technically unused as `spawn_ant` uses `self.pos`.
    pub fn respawn_ant(&mut self, ant_key: AntKey, _new_pos: Vec2, map: &mut GameMap) {
        // First, ensure the ant to be "respawned" (i.e., replaced) exists in this colony.
        if self.ants.contains_key(ant_key) {
            // Despawn the old ant.
            self.despawn_ant(ant_key, map);
            // Spawn a new ant at the colony's nest position.
            self.spawn_ant(map);
        } else {
            eprintln!(
                "Warning: AntKey {:?} not found in colony {} during respawn attempt (despawn/spawn).",
                ant_key, self.colony_id
            );
        }
    }

    /// Get the pheromone level for a specific channel at a specific tile coordinate.
    pub fn get_pheromone_channel_at(&self, x: usize, y: usize, channel_index: usize) -> f32 {
        if channel_index < self.pheromones.len() {
            let channel = &self.pheromones[channel_index];
            if x < channel.width as usize && y < channel.height as usize {
                return channel.data[y][x];
            }
        }
        0.0 // Return 0 if channel index or coordinates are out of bounds
    }

    pub fn is_dead(&self) -> bool {
        self.ants.is_empty()
    }
}
