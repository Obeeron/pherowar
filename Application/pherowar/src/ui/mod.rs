pub mod components;
pub mod events;

pub use ui_manager::UIManager;

mod ui_manager;

// Base sizes (logical points)
pub const BASE_PADDING: f32 = 6.0;
pub const BASE_SPACING: f32 = 6.0;
pub const BASE_BUTTON_WIDTH: f32 = 60.0;
pub const BASE_BUTTON_HEIGHT: f32 = 32.0;
pub const BASE_ICON_SIZE: f32 = 36.0;
