use super::{
    ANT_ATTACK_DAMAGE, ANT_LENGTH, ANT_SPEED, COLONY_NEST_SIZE, MAX_ANT_PROCESSING_TIME,
    MAX_PHEROMONE_AMOUNT, SENSE_MAX_ANGLE, SENSE_MAX_DISTANCE, SENSE_NUM_SAMPLES,
    pheromone::PheromoneChannel,
};
use super::{MAX_ANT_LONGEVITY, THINK_INTERVAL, Timer};
use crate::player::PlayerConnection;
use crate::simulation::{Colony, GameMap, Terrain};

use shared::PHEROMONE_CHANNEL_COUNT;
use shared::{AntInput, AntOutput, MEMORY_SIZE, util::fast_sin_cos};

use anyhow::Result;
use macroquad::prelude::{Vec2, rand};
use slotmap::{Key, new_key_type};
use std::collections::HashMap;
use std::f32;

new_key_type! {
    /// Key for ant slotmap.
    pub struct AntKey;
}

/// Reference to an ant.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AntRef {
    pub key: AntKey,
    pub colony_id: u32,
}

/// Opponent ant in a fight.
#[derive(Debug, Clone)]
pub struct FightOpponent {
    pub ant_ref: AntRef,
    pub orientation: f32,
}

/// State of an ant.
pub struct Ant {
    pub ant_ref: AntRef,

    pub pos: Vec2,
    pub rotation: f32,
    pub speed: f32,
    pub longevity: f32,
    pub is_on_colony: bool,
    pub is_on_food: bool,
    pub carrying_food: bool,
    pub fight_opponents: Vec<FightOpponent>,
    pub memory: [u8; MEMORY_SIZE],

    pub think_timer: Timer,
    pub try_attack: bool,
}

impl Ant {
    /// Create a new ant.
    pub fn new(pos: Vec2, colony_id: u32) -> Self {
        let ant_ref = AntRef {
            key: AntKey::null(),
            colony_id,
        };

        // Start think timer with a random value
        let initial_think_timer_value = rand::gen_range(0.0, THINK_INTERVAL);
        let think_timer = Timer::new(THINK_INTERVAL, initial_think_timer_value);

        Self {
            pos,
            rotation: rand::gen_range(0.0, f32::consts::TAU),
            speed: ANT_SPEED,
            ant_ref,
            think_timer,
            carrying_food: false,
            is_on_colony: true,
            is_on_food: false,
            memory: [0u8; MEMORY_SIZE],   // zero-initialized
            longevity: MAX_ANT_LONGEVITY, // start at max
            fight_opponents: Vec::new(),  // Initialize active_fights to an empty vector
            try_attack: false,            // initialize
        }
    }

    /// Update ant state and behavior.
    pub fn update(
        &mut self,
        colony_pos: &Vec2,
        map: &mut GameMap,
        pheromones: &mut [PheromoneChannel],
        player_connection: &mut PlayerConnection,
        other_colonies: &mut HashMap<u32, Colony>,
        dt: f32,
    ) {
        if self.is_dead() {
            return;
        }

        self.think_timer.update(dt);

        if !self.think_timer.is_ready() {
            // Handle autopilot tick
            // During this tick, if the ant finds an enemy ant in the same cell and wants to fight,
            // the enemy ant will be attacked and the ant will be forced to think during this tick

            // Check for enemy in the current cell to initiate a fight
            if self.try_attack && !self.is_fighting() {
                // Ant is not currently fighting and wanted to fight last think tick
                // Check for ennemy in the current cell to initiate a fight

                let x = self.pos.x.floor() as usize;
                let y = self.pos.y.floor() as usize;
                if let Some(opponent_ref) = map.get_enemy_ant_at(x, y, self.ant_ref.colony_id) {
                    // Found an enemy ant in the same cell, initiate a fight
                    if self.try_initiate_fight(&opponent_ref, other_colonies) {
                        self.think_timer.force_ready();
                    }
                }
            }
        }

        if self.think_timer.is_ready() {
            // Handle think tick
            // During this tick, the ant perceives the environment, thinks (player update call), and applies pheromones

            self.think_timer.wrap();

            // Perceive the environment
            let (ant_input, perceived) = self.perceive(map, pheromones, colony_pos);

            // Call the player update function and sanitize the output
            let sanitized_ouput = match self.think(ant_input, player_connection) {
                Ok(mut output) => {
                    self.sanitize_output(&mut output);
                    output
                }
                Err(e) => {
                    eprintln!(
                        "Ignored think tick for {:?} because of error: {:?}",
                        self.ant_ref.key, e
                    );
                    return;
                }
            };

            // Apply pheromones
            self.apply_pheromones(sanitized_ouput.pheromone_amounts, pheromones);
            self.try_attack = sanitized_ouput.try_attack;
            if self.try_attack && !self.is_fighting() {
                if let Some(mut perceived) = perceived {
                    self.try_initiate_fight(&mut perceived, other_colonies);
                }
            }

            // Update orientation
            if self.is_fighting() {
                // Fighting -> Handle fight
                self.handle_fight(other_colonies);
            } else {
                // Not fighting -> Update rotation
                self.rotation =
                    (self.rotation + sanitized_ouput.turn_angle).rem_euclid(f32::consts::TAU);
            }
        }

        if !self.is_fighting() {
            // Not fighting -> Move
            self.update_position(map, dt);
        }
    }

    fn handle_fight(&mut self, other_colonies: &mut HashMap<u32, Colony>) -> bool {
        // Handle fight logic here
        // For example, you can check if the ant is still alive and update its state accordingly
        // This is a placeholder for the actual fight handling logic

        // Attack until either a hit succeeds or there are no more opponents.
        while !self.fight_opponents.is_empty() {
            let fight_opponent = self.fight_opponents[0].clone();
            if self.try_attack(&fight_opponent, other_colonies) {
                return true;
            }
        }
        false
    }

    fn rejuvenate_by(&mut self, amount: f32) {
        // Increase longevity by a certain amount, but not exceeding the maximum
        self.longevity = (self.longevity + amount).min(MAX_ANT_LONGEVITY);
    }
    /// Restore ant longevity.
    pub fn rejuvenate(&mut self) {
        self.longevity = MAX_ANT_LONGEVITY;
    }

    fn perceive(
        &mut self,
        map: &mut GameMap,
        pheromones: &[PheromoneChannel],
        colony_pos: &Vec2,
    ) -> (AntInput, Option<AntRef>) {
        // Initialize AntInput
        let mut ant_input = AntInput {
            is_carrying_food: self.carrying_food,
            is_on_colony: self.is_on_colony,
            is_on_food: self.is_on_food,
            longevity: self.longevity,
            pheromone_senses: [(0.0, 0.0); PHEROMONE_CHANNEL_COUNT],
            cell_sense: [0.0; PHEROMONE_CHANNEL_COUNT],
            wall_sense: (0.0, -1.0),
            food_sense: (0.0, -1.0),
            colony_sense: (0.0, -1.0),
            enemy_sense: (0.0, -1.0),
            is_fighting: self.is_fighting(),
        };

        let x = self.pos.x.floor() as usize;
        let y = self.pos.y.floor() as usize;

        // Sense pheromones in current cell
        for channel in 0..PHEROMONE_CHANNEL_COUNT {
            if y < pheromones[channel].data.len() && x < pheromones[channel].data[y].len() {
                ant_input.cell_sense[channel] = pheromones[channel].data[y][x];
            }
        }

        let mut attackable_enemy_ref: Option<AntRef> = None;
        // Sense enemy in current cell (without using other_colonies)
        if let Some(ant_ref) = map.get_enemy_ant_at(x, y, self.ant_ref.colony_id) {
            // Found an enemy ant in the same cell
            ant_input.enemy_sense = (0.0, 0.0);
            attackable_enemy_ref = Some(ant_ref.clone());
        }

        // Raycast to colony
        let dx = colony_pos.x - self.pos.x;
        let dy = colony_pos.y - self.pos.y;
        let angle_to_colony = dy.atan2(dx);
        let dist_to_colony_sq = dx * dx + dy * dy;
        if dist_to_colony_sq <= SENSE_MAX_DISTANCE * SENSE_MAX_DISTANCE {
            let (blocked, dist) =
                map.raycast_angle(self.pos, angle_to_colony, dist_to_colony_sq.sqrt());
            if !blocked {
                ant_input.colony_sense = (angle_to_colony - self.rotation, dist);
            }
        }

        // Sense the environment in the ant's perception cone by sampling at random angles and distances
        for _ in 0..SENSE_NUM_SAMPLES {
            let angle_offset = rand::gen_range(-SENSE_MAX_ANGLE, SENSE_MAX_ANGLE);
            let angle = self.rotation + angle_offset;
            let random_dist = rand::gen_range(1.0, SENSE_MAX_DISTANCE);

            // Sense wall or map edge
            let (blocked, wall_dist) = map.raycast_angle(self.pos, angle, random_dist);
            if blocked {
                if wall_dist < ant_input.wall_sense.1 || ant_input.wall_sense.1 < 0.0 {
                    ant_input.wall_sense = (angle_offset, wall_dist);
                }
                continue;
            }

            let (sin_a, cos_a) = fast_sin_cos(angle);
            let sample_x = self.pos.x + cos_a * random_dist;
            let sample_y = self.pos.y + sin_a * random_dist;
            let xi = sample_x as isize;
            let yi = sample_y as isize;
            if !(xi >= 0 && yi >= 0 && xi < map.width as isize && yi < map.height as isize) {
                continue;
            }
            let dist: f32 =
                ((self.pos.x - sample_x).powi(2) + (self.pos.y - sample_y).powi(2)).sqrt();

            // Sense pheromones
            for channel in 0..PHEROMONE_CHANNEL_COUNT {
                let intensity = pheromones[channel].data[yi as usize][xi as usize];
                if intensity > ant_input.pheromone_senses[channel].1 {
                    ant_input.pheromone_senses[channel] = (angle_offset, intensity);
                }
            }

            // Sense enemies
            if let Some(ant_ref) =
                map.get_enemy_ant_at(xi as usize, yi as usize, self.ant_ref.colony_id)
            {
                if dist < ant_input.enemy_sense.1 || ant_input.enemy_sense.1 < 0.0 {
                    ant_input.enemy_sense = (angle_offset, dist);

                    if dist <= ANT_LENGTH {
                        attackable_enemy_ref = Some(ant_ref.clone());
                    }
                }
            }

            match map.get_terrain_at(xi as usize, yi as usize) {
                Some(Terrain::Food(_)) => {
                    if dist < ant_input.food_sense.1 || ant_input.food_sense.1 < 0.0 {
                        ant_input.food_sense = (angle_offset, dist);
                    }
                }
                _ => {}
            }
        }

        (ant_input, attackable_enemy_ref)
    }

    fn think(
        &mut self,
        ant_input: AntInput,
        player_connection: &mut PlayerConnection,
    ) -> Result<AntOutput> {
        let req = shared::AntRequest {
            input: ant_input,
            memory: self.memory,
        };

        let start_time = std::time::Instant::now();
        let resp_result = player_connection.player_update(req);
        let elapsed_time = start_time.elapsed().as_nanos();

        if elapsed_time > MAX_ANT_PROCESSING_TIME {
            self.die();
            return Err(anyhow::anyhow!(
                "{:?} processing timed out. Took too long to process ({:}ns > {:}ns).",
                self.ant_ref,
                elapsed_time,
                MAX_ANT_PROCESSING_TIME
            ));
        }

        let resp = resp_result?;
        self.memory = resp.memory;
        Ok(resp.output)
    }

    fn apply_pheromones(
        &mut self,
        pheromones_layed: [f32; PHEROMONE_CHANNEL_COUNT],
        pheromones_channels: &mut [PheromoneChannel],
    ) {
        let cell_x = self.pos.x.floor() as usize;
        let cell_y = self.pos.y.floor() as usize;

        for (idx, &amount) in pheromones_layed.iter().enumerate() {
            if amount > 0.0 && idx < PHEROMONE_CHANNEL_COUNT {
                pheromones_channels[idx].lay(cell_x, cell_y, amount);
            }
        }
    }

    /// Attack the target ant if within range and alive.
    /// Returns true if the hit was successful.
    fn try_attack(
        &mut self,
        fight_opponent: &FightOpponent,
        other_colonies: &mut HashMap<u32, Colony>,
    ) -> bool {
        // Use stored orientation to face the opponent
        self.rotation = fight_opponent.orientation;

        let target_colony_id = fight_opponent.ant_ref.colony_id;
        let target_key = fight_opponent.ant_ref.key;

        let mut target_is_alive_and_found = false;
        let mut hit_successful = false;

        if let Some(target_colony_mut) = other_colonies.get_mut(&target_colony_id) {
            if let Some(target) = target_colony_mut.ants.get_mut(target_key) {
                let distance_sq = self.pos.distance_squared(target.pos);
                if !target.is_dead() && distance_sq <= ANT_LENGTH * ANT_LENGTH {
                    target_is_alive_and_found = true;

                    // Attack the target
                    target.take_damage(ANT_ATTACK_DAMAGE);
                    hit_successful = true;

                    if target.is_dead() {
                        // Killed the target
                        self.rejuvenate_by(MAX_ANT_LONGEVITY - self.longevity / 2.0); // Rejuvenate half of the longevity
                        self.remove_opponent(target_key); // Remove dead opponent
                    }
                }
            }
        }

        if !target_is_alive_and_found {
            // Target is already dead (probably removed from map)
            // or too far away (respawned when wall placed)
            self.remove_opponent(target_key);
        }

        return hit_successful;
    }

    /// Moves the ant to a new position and updates its registration in the spatial index.
    pub fn move_to_pos(&mut self, map: &mut GameMap, new_pos: Vec2) {
        let old_pos = self.pos; // Store current position before updating

        // Determine current and new cell coordinates for map operations
        let old_cell_x = old_pos.x.floor() as isize;
        let old_cell_y = old_pos.y.floor() as isize;
        let new_cell_x = new_pos.x.floor() as isize;
        let new_cell_y = new_pos.y.floor() as isize;

        // Update the ant's internal position state.
        self.pos = new_pos;
        // Only update map registration if the ant is actually changing cells.
        if old_cell_x != new_cell_x || old_cell_y != new_cell_y {
            // Unregister from the old cell.
            // It's important to use old_pos here, as self.pos will be updated shortly.
            if !map.unregister_ant_from_cell(&self.ant_ref, old_pos) {
                // This warning indicates a potential desync if an ant wasn't where it thought it was.
                eprintln!(
                    "Warning: Ant {:?} was not found in its expected old cell ({:.2},{:.2}) during move_to_pos. Ant's internal old_pos: ({:.2},{:.2})",
                    self.ant_ref,
                    old_pos.x.floor(),
                    old_pos.y.floor(),
                    old_pos.x,
                    old_pos.y
                );
            }

            // Register in the new cell, but only if it changed cells.
            // If it stayed in the same cell, it should still be registered there from before (or if it's a new ant, spawn_ant handles initial registration).
            // However, to be robust against potential desyncs or if an ant was somehow unregistered, we can re-register.
            // If the cell hasn't changed, map.register_ant_in_cell will just re-insert, which is fine for a HashSet.
            map.register_ant_in_cell(&self.ant_ref, self.pos);
        }

        // If an ant moves *within* the same cell, its registration in ants_in_cell doesn't need to change.
        // The logic above handles changing cells. If it stays in the same cell, no map calls are made here.
    }

    fn update_position(&mut self, map: &mut GameMap, dt: f32) {
        let (dy, dx) = fast_sin_cos(self.rotation);
        let mut speed = self.speed;
        if self.carrying_food {
            speed *= super::ANT_SLOWNESS_WITH_FOOD;
        }
        let next_x_float = self.pos.x + dx * speed * dt;
        let next_y_float = self.pos.y + dy * speed * dt;

        // Check for NaN before passing to move_to_pos
        if next_x_float.is_nan() || next_y_float.is_nan() {
            eprintln!(
                "Warning: Ant {:?} calculated NaN next position (dx:{:.2}, dy:{:.2}, rot:{:.2}). Movement aborted.",
                self.ant_ref, dx, dy, self.rotation
            );
            // Ant's self.pos remains unchanged, and it stays in its current cell in ants_in_cell.
            // This effectively means the ant doesn't move this tick if its calculations result in NaN.
            return;
        }

        let w = map.width as f32;
        let h = map.height as f32;

        let next_cell_x_isize = next_x_float.floor() as isize;
        let next_cell_y_isize = next_y_float.floor() as isize;

        let blocked = map
            .get_terrain_at(next_cell_x_isize as usize, next_cell_y_isize as usize)
            .map_or(true, |terrain| terrain == &Terrain::Wall);

        if !blocked {
            // Call the new centralized function to update position and spatial index
            self.move_to_pos(map, Vec2::new(next_x_float, next_y_float)); // Removed colony_id
        } else {
            // Collision handling logic (rotation)
            let try_rotate = |angle: f32| -> bool {
                let (dy_r, dx_r) = fast_sin_cos(self.rotation + angle);
                let tx = self.pos.x + dx_r * self.speed * dt;
                let ty = self.pos.y + dy_r * self.speed * dt;
                if tx < 0.0 || tx >= w || ty < 0.0 || ty >= h {
                    return false;
                }
                let mx = tx.floor() as isize;
                let my = ty.floor() as isize;
                map.get_terrain_at(mx as usize, my as usize)
                    .map_or(false, |terrain| terrain != &Terrain::Wall)
            };

            let cw_clear = try_rotate(f32::consts::FRAC_PI_4);
            let ccw_clear = try_rotate(-f32::consts::FRAC_PI_4);

            if cw_clear && !ccw_clear {
                self.rotation = (self.rotation + f32::consts::FRAC_PI_4) % f32::consts::TAU;
            } else if ccw_clear && !cw_clear {
                self.rotation = (self.rotation - f32::consts::FRAC_PI_4) % f32::consts::TAU;
            } else if cw_clear && ccw_clear {
                self.rotation = (self.rotation + f32::consts::FRAC_PI_4) % f32::consts::TAU;
            } else {
                // Both blocked, rotate 180
                self.rotation = (self.rotation + f32::consts::PI) % f32::consts::TAU;
            }
        }
    }

    pub fn check_colony(&mut self, colony_pos: &Vec2) {
        let dx = self.pos.x - colony_pos.x;
        let dy = self.pos.y - colony_pos.y;
        if (dx * dx + dy * dy) <= COLONY_NEST_SIZE * COLONY_NEST_SIZE / 4.0 {
            if !self.is_on_colony {
                // Force a think tick when the ant enters colony
                self.think_timer.force_ready();
            }
            self.is_on_colony = true;
        } else {
            self.is_on_colony = false;
        }
    }

    pub fn check_food(&mut self, map: &mut GameMap) {
        let x = self.pos.x.floor() as usize;
        let y = self.pos.y.floor() as usize;
        match map.get_terrain_at(x, y) {
            Some(Terrain::Food(_)) => {
                if !self.is_on_food {
                    // Force a think tick when the ant enters food
                    self.think_timer.force_ready();
                }
                if !self.carrying_food {
                    map.take_food_at(x, y);
                    self.carrying_food = true;
                    self.rejuvenate();
                }

                // Re-check terrain after taking food to correctly set is_on_food
                if let Some(Terrain::Food(_)) = map.get_terrain_at(x, y) {
                    self.is_on_food = true;
                } else {
                    self.is_on_food = false;
                }
            }
            _ => {
                self.is_on_food = false;
            }
        }
    }

    pub fn take_damage(&mut self, damage: f32) {
        self.longevity = (self.longevity - damage).max(0.0);
    }

    /// Returns true if ant is dead.
    pub fn is_dead(&self) -> bool {
        self.longevity <= 0.0
    }

    /// Add an opponent to the fight_opponents list. Returns true if added.
    pub fn try_add_opponent(
        &mut self,
        opponent_ant_ref: &AntRef,
        orientation_to_opponent: f32,
    ) -> bool {
        if self
            .fight_opponents
            .iter()
            .any(|fo| fo.ant_ref == *opponent_ant_ref)
        {
            return false;
        }

        // New opponent, add to the back of the opponents list
        self.fight_opponents.push(FightOpponent {
            ant_ref: opponent_ant_ref.clone(),
            orientation: orientation_to_opponent,
        });
        true
    }

    pub fn try_initiate_fight(
        &mut self,
        opponent_ref: &AntRef,
        other_colonies: &mut HashMap<u32, Colony>,
    ) -> bool {
        let opponent = match get_ant_by_ref(&opponent_ref, other_colonies) {
            Some(opponent) => opponent,
            None => {
                // Opponent is dead or not found
                return false;
            }
        };

        // Compute the angle to the opponent
        let dx = opponent.pos.x - self.pos.x;
        let dy = opponent.pos.y - self.pos.y;
        let orientation_to_opponent = dy.atan2(dx);
        let distance_sq = dx * dx + dy * dy;
        if distance_sq > ANT_LENGTH * ANT_LENGTH {
            // Too far to initiate a fight
            return false;
        }

        if !self.try_add_opponent(&opponent.ant_ref, orientation_to_opponent) {
            eprintln!(
                "Warning: Ant {:?} tried to add opponent {:?} but it was already present.",
                self.ant_ref, opponent.ant_ref
            );
            return false;
        }

        // Add the opponent to the fight_opponents list
        if !opponent.try_add_opponent(&self.ant_ref, orientation_to_opponent + f32::consts::PI) {
            eprintln!(
                "Warning: Unexpected faiure while trying to add Ant {:?} to the oppenent's {:?} fight.",
                opponent.ant_ref, self.ant_ref
            );
            self.remove_opponent(opponent.ant_ref.key);
            return false;
        }

        return true;
    }

    // Method for the simulation to tell this ant to remove an opponent
    pub fn remove_opponent(&mut self, opponent_key: AntKey) {
        self.fight_opponents
            .retain(|fo| fo.ant_ref.key != opponent_key);
    }

    pub fn is_fighting(&self) -> bool {
        !self.fight_opponents.is_empty()
    }

    fn die(&mut self) {
        self.longevity = 0.0;
    }

    fn sanitize_output(&self, output: &mut AntOutput) {
        // Sanitize pheromone amounts
        for amount in &mut output.pheromone_amounts {
            if amount.is_nan() {
                *amount = 0.0; // Default to no pheromone
                eprintln!(
                    "Warning: Ant {:?} received NaN pheromone amount. Defaulting to 0.0.",
                    self.ant_ref
                );
            } else {
                *amount = amount.clamp(0.0, MAX_PHEROMONE_AMOUNT);
            }
        }

        // Sanitize turn angle
        if output.turn_angle.is_nan() {
            output.turn_angle = 0.0; // Default to no rotation
            eprintln!(
                "Warning: Ant {:?} received NaN turn_angle. Defaulting to 0.0.",
                self.ant_ref
            );
        } else {
            output.turn_angle = output.turn_angle.rem_euclid(f32::consts::TAU);
        }
    }
}

fn get_ant_by_ref<'a>(
    ant_ref: &'a AntRef,
    other_colonies: &'a mut HashMap<u32, Colony>,
) -> Option<&'a mut Ant> {
    if let Some(colony) = other_colonies.get_mut(&ant_ref.colony_id) {
        if let Some(ant) = colony.ants.get_mut(ant_ref.key) {
            if !ant.is_dead() {
                return Some(ant);
            }
        }
    }
    None
}
