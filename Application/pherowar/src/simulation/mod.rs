pub mod ant;
mod colony;
mod map;
mod pheromone;
mod raycast;
mod sim;
mod timer;

// Re-export key types for easier imports
pub use ant::AntRef;
pub use colony::Colony;
pub use map::GameMap;
pub use map::Terrain;
pub use raycast::RaycastCache;
pub use sim::Simulation;
pub use timer::Timer;

// Time constants
pub const MIN_TIME_MULTIPLIER: f32 = 0.1;
pub const MAX_TIME_MULTIPLIER: f32 = 2.0;
pub const ANT_SPAWN_INTERVAL: f32 = 0.3;

// Simulation constants
pub const DEFAULT_FOOD_AMOUNT: u32 = 50;
pub const COLONY_NEST_SIZE: f32 = 8.0;
pub const MAX_COLONIES: usize = 5;
pub const ANT_SPAWN_FOOD_COST: u32 = 5;
pub const MAX_PHEROMONE_AMOUNT: f32 = 255.0;

// Map size defaults
pub const DEFAULT_MAP_WIDTH: u32 = 360;
pub const DEFAULT_MAP_HEIGHT: u32 = 200;

// Ant behavior constants
pub const THINK_INTERVAL: f32 = 1.5 / ANT_SPEED; // How often the ant thinks (in seconds) : Once per cell
pub const ANT_LENGTH: f32 = 1.0;
pub const ANT_SPEED: f32 = 4.0; // How much the ant moves in 1 second at 1x speed
pub const ANT_SLOWNESS_WITH_FOOD: f32 = 0.9; // Ants are 10% slower when carrying food
pub const SENSE_MAX_ANGLE: f32 = std::f32::consts::FRAC_PI_4; // 45 degrees
pub const SENSE_MAX_DISTANCE: f32 = 10.0;
pub const SENSE_NUM_SAMPLES: usize = 32;
// pub const MAX_ANT_AGE: f32 = 200.0; // in seconds, 200 is enough for 1.5 map length walk
pub const MAX_ANT_LONGEVITY: f32 = 300.0; // in seconds, 200 is enough for 1.5 map length walk
pub const ANT_ATTACK_DAMAGE: f32 = 5.0;
pub const MAX_ANT_PROCESSING_TIME: u128 = 1500000; // Max time in nanos for an ant to be processed by the player connection

// Pheromone decay interval (seconds)
pub const PHEROMONE_DECAY_INTERVAL: f32 = 1.0; // 1 time every 1 seconds
