use crate::config::PlayerConfig;
use crate::engine::Renderer;
use crate::simulation::Simulation;
use macroquad::prelude::{
    KeyCode, MouseButton, Vec2, is_key_down, is_mouse_button_down, mouse_wheel,
};

use crate::editor::color_palette::ColorPalette;
use crate::editor::symmetry_mode::SymmetryMode;
// Keep only one import for ToolType, directly from its definition path
use crate::editor::tool_type::ToolType;

// Import functions from the tools module
use crate::editor::tools::colony_tool::{
    apply_colony, is_colony_tool_draggable, render_colony_preview,
};
use crate::editor::tools::food_tool::{apply_food, is_food_tool_draggable, render_food_preview};
use crate::editor::tools::wall_tool::{apply_wall, is_wall_tool_draggable, render_wall_preview};

/// Minimum allowed tool size
pub const MIN_TOOL_SIZE: f32 = 1.0;
/// Maximum allowed tool size
pub const MAX_TOOL_SIZE: f32 = 100.0;
/// Minimum distance for drag application.
pub const TOOL_DRAG_THRESHOLD: f32 = 0.1;

/// Manages editor tools, input, and state.
pub struct EditorManager {
    current_tool_type: Option<ToolType>,
    tool_size: f32,
    is_removing: bool,                   // True if right mouse button is pressed
    last_drag_pos: Option<Vec2>,         // For continuous tool application
    current_player_index: Option<usize>, // 0 for placeholder, 1-based for players
    pub color_palette: ColorPalette,
    pub symmetry_mode: SymmetryMode,
    player_configs: Vec<PlayerConfig>, // Available player configurations
}

impl EditorManager {
    /// Creates a new `EditorManager`.
    pub fn new(player_configs_ref: &Vec<PlayerConfig>) -> Self {
        let initial_player_index = if !player_configs_ref.is_empty() {
            Some(1) // Default to first player
        } else {
            Some(0) // Default to placeholder if no players
        };
        Self {
            current_tool_type: None,
            tool_size: 10.0, // Default tool size
            is_removing: false,
            last_drag_pos: None,
            current_player_index: initial_player_index,
            color_palette: ColorPalette::new(),
            symmetry_mode: SymmetryMode::None,
            player_configs: player_configs_ref.clone(),
        }
    }

    /// Gets the currently active tool.
    pub fn current_tool(&self) -> Option<ToolType> {
        self.current_tool_type
    }

    /// Sets the active tool.
    pub fn set_tool(&mut self, tool_opt: Option<ToolType>) {
        self.current_tool_type = tool_opt;
    }

    /// Gets the current tool size.
    pub fn tool_size(&self) -> f32 {
        self.tool_size
    }

    /// Gets the index of the currently selected player or placeholder.
    pub fn current_player_index(&self) -> Option<usize> {
        self.current_player_index
    }

    /// Sets the current player/placeholder.
    pub fn set_player(&mut self, index: Option<usize>) {
        self.current_player_index = index;
    }

    /// Sets the tool size, clamping it within min/max bounds.
    pub fn set_tool_size(&mut self, size: f32) {
        self.tool_size = size.clamp(MIN_TOOL_SIZE, MAX_TOOL_SIZE);
    }

    /// Handles user input for the editor.
    pub fn handle_input(
        &mut self,
        simulation: &mut Simulation,
        renderer: &mut Renderer,
        world_pos: Vec2,
    ) -> bool {
        if self.current_tool_type.is_none() {
            return false;
        }

        let ctrl_pressed = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);

        if ctrl_pressed {
            let wheel = mouse_wheel().1;
            if wheel != 0.0 {
                let speed = (self.tool_size / 10.0).max(1.0);
                self.set_tool_size(self.tool_size - wheel * speed);
                return true; // Input handled
            }
            // Ctrl pressed but no wheel: let other bindings proceed.
            return false;
        }

        // Update removal state based on right mouse button.
        self.is_removing = is_mouse_button_down(MouseButton::Right);

        if is_mouse_button_down(MouseButton::Left) || is_mouse_button_down(MouseButton::Right) {
            // Determine if tool is draggable.
            let is_tool_draggable = match self.current_tool_type {
                Some(ToolType::Food) => is_food_tool_draggable(),
                Some(ToolType::Wall) => is_wall_tool_draggable(),
                Some(ToolType::Colony) => is_colony_tool_draggable(),
                None => false, // Should be caught by early exit
            };

            let apply_this_frame;
            if is_tool_draggable {
                // Apply draggable tool if moved beyond threshold or first click.
                apply_this_frame = match self.last_drag_pos {
                    Some(last_pos) => {
                        (world_pos - last_pos).length_squared() > TOOL_DRAG_THRESHOLD.powi(2)
                    }
                    None => true,
                };
            } else {
                // Apply non-draggable tool only on initial press.
                apply_this_frame = self.last_drag_pos.is_none();
            }

            if apply_this_frame {
                if self.apply_active_tool_with_symmetry(world_pos, simulation) {
                    renderer.mark_dirty(); // Mark renderer dirty if changes were made
                }
            }
            // Store current position for next frame's drag check or to prevent re-application.
            self.last_drag_pos = Some(world_pos);
            return true; // Input handled
        } else {
            // No mouse buttons down: reset drag state and ensure not removing.
            self.last_drag_pos = None;
            if self.is_removing {
                // Reset if it was true.
                self.is_removing = false;
            }
        }
        false // No relevant input handled by this path
    }

    /// Applies the active tool at `primary_world_pos` and symmetric positions.
    fn apply_active_tool_with_symmetry(
        &mut self,
        primary_world_pos: Vec2,
        simulation: &mut Simulation,
    ) -> bool {
        let mut overall_change = false;

        // Primary application
        if self.dispatch_tool_action(primary_world_pos, simulation) {
            overall_change = true;
        }

        // Apply to symmetric positions if symmetry is enabled.
        if self.symmetry_mode != SymmetryMode::None {
            let map_w = simulation.map.width as f32;
            let map_h = simulation.map.height as f32;

            for sym_pos in self
                .symmetry_mode
                .symmetric_positions(primary_world_pos, map_w, map_h)
            {
                // Avoid re-applying to the exact same primary position.
                if (sym_pos - primary_world_pos).length_squared() < 0.001 {
                    continue;
                }

                if self.dispatch_tool_action(sym_pos, simulation) {
                    overall_change = true;
                }
            }
        }
        overall_change // True if any application (primary or symmetric) occurred
    }

    /// Dispatches the current tool action to the appropriate handler.
    fn dispatch_tool_action(&mut self, world_pos: Vec2, simulation: &mut Simulation) -> bool {
        match self.current_tool_type {
            Some(ToolType::Food) => {
                apply_food(world_pos, self.tool_size, self.is_removing, simulation)
            }
            Some(ToolType::Wall) => {
                apply_wall(world_pos, self.tool_size, self.is_removing, simulation)
            }
            Some(ToolType::Colony) => apply_colony(
                world_pos,
                self.is_removing,
                self.current_player_index,
                &self.player_configs,
                &mut self.color_palette,
                simulation,
            ),
            None => false,
        }
    }

    /// Renders the preview for the currently active tool.
    pub fn render_tool_preview(&self, world_pos: Vec2) {
        match self.current_tool_type {
            Some(ToolType::Food) => {
                render_food_preview(world_pos, self.tool_size, self.is_removing)
            }
            Some(ToolType::Wall) => {
                render_wall_preview(world_pos, self.tool_size, self.is_removing)
            }
            Some(ToolType::Colony) => {
                render_colony_preview(world_pos, self.is_removing, self.current_player_index)
            }
            None => {} // No tool, no preview
        }
    }
}
