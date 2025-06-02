mod camera;
mod rendering;

pub use camera::GameCamera;
pub use rendering::Renderer;
pub use rendering::CameraAction; // Add this line

use macroquad::prelude::Color;

// Define the constant for wall brightness variation
pub const WALL_BRIGHTNESS_VARIATION: f32 = 1.0;

// Rendering constants moved from rendering.rs
pub const WALL_BASE_COLOR_VAL: u32 = 0x504945; // Brighter base gray (Gruvbox bg2)
pub const WALL_EDGE_BRIGHTNESS_BOOST: f32 = 0.10;
pub const WALL_EDGE_SATURATION_BOOST: f32 = 0.15;
pub const CHANNEL_COLORS: [Color; 8] = [
    Color::new(1.0, 0.0, 0.0, 1.0), // red
    Color::new(0.0, 1.0, 0.0, 1.0), // green
    Color::new(0.0, 0.0, 1.0, 1.0), // blue
    Color::new(1.0, 1.0, 0.0, 1.0), // yellow
    Color::new(1.0, 0.0, 1.0, 1.0), // magenta
    Color::new(0.0, 1.0, 1.0, 1.0), // cyan
    Color::new(1.0, 0.5, 0.0, 1.0), // orange
    Color::new(0.5, 0.0, 1.0, 1.0), // purple
];
