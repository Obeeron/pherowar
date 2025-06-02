use rkyv::{Archive, Deserialize, Serialize};

pub const MEMORY_SIZE: usize = 32;
pub const PHEROMONE_CHANNEL_COUNT: usize = 8;

#[derive(Archive, Serialize, Deserialize, Debug, Clone, Copy)]
#[repr(C)]
pub struct AntInput {
    pub is_carrying_food: bool,
    pub is_on_colony: bool,
    pub is_on_food: bool,
    pub pheromone_senses: [(f32, f32); PHEROMONE_CHANNEL_COUNT], // angle, intensity
    pub cell_sense: [f32; PHEROMONE_CHANNEL_COUNT],              // intensity
    pub wall_sense: (f32, f32),                                  // angle, distance
    pub food_sense: (f32, f32),                                  // angle, distance
    pub colony_sense: (f32, f32),                                // angle, distance
    pub enemy_sense: (f32, f32),                                 // angle, distance
    pub longevity: f32,
    pub is_fighting: bool,
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone, Copy)]
#[repr(C)]
pub struct AntOutput {
    pub turn_angle: f32,
    pub pheromone_amounts: [f32; PHEROMONE_CHANNEL_COUNT],
    pub try_attack: bool,
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone, Copy)]
#[repr(C)]
pub struct AntRequest {
    pub input: AntInput,
    pub memory: [u8; MEMORY_SIZE],
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone, Copy)]
#[repr(C)]
pub struct AntResponse {
    pub output: AntOutput,
    pub memory: [u8; MEMORY_SIZE],
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone, Copy)]
#[repr(C)]
pub struct PlayerSetup {
    pub decay_rates: [f32; PHEROMONE_CHANNEL_COUNT],
}
