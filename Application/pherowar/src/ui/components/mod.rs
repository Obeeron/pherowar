// Components for the UI system
mod ant_status_bar;
mod colony_options;
mod debug_panel;
mod dialog;
mod tool_size_slider;
mod top_panel;
mod visual_options;

// Export components
pub use ant_status_bar::AntStatusBar;
pub use colony_options::ColonyOptions;
pub use debug_panel::DebugPanel;
pub use dialog::{DialogPopup, DialogPopupMode, DialogPopupResult};
pub use tool_size_slider::ToolSizeSlider;
pub use top_panel::TopPanel;
pub use visual_options::{PheromoneDisplayMode, VisualOptionsPanel};
