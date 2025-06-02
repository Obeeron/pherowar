use crate::simulation::Simulation;
use macroquad::prelude::Vec2;

/// Helper function to apply an action in a circular area around a center point.
/// The `apply_on_tile` closure takes (tile_x, tile_y, simulation) and returns true if an action was performed.
pub fn apply_action_in_circular_area<F>(
    center_world_pos: Vec2,
    tool_size: f32,
    simulation: &mut Simulation,
    mut apply_on_tile: F,
) -> bool
where
    F: FnMut(usize, usize, &mut Simulation) -> bool, // tile_x, tile_y, simulation -> bool (changed)
{
    let mut action_performed_overall = false;
    let radius = tool_size / 2.0;
    let r_squared = radius * radius;

    let start_x = (center_world_pos.x - radius).floor() as i32;
    let start_y = (center_world_pos.y - radius).floor() as i32;
    let end_x = (center_world_pos.x + radius).ceil() as i32;
    let end_y = (center_world_pos.y + radius).ceil() as i32;

    let map_wi = simulation.map.width as i32;
    let map_hi = simulation.map.height as i32;

    for y_idx_i32 in start_y..=end_y {
        if y_idx_i32 < 0 || y_idx_i32 >= map_hi {
            continue;
        }
        for x_idx_i32 in start_x..=end_x {
            if x_idx_i32 < 0 || x_idx_i32 >= map_wi {
                continue;
            }

            let tile_x = x_idx_i32 as usize;
            let tile_y = y_idx_i32 as usize;
            let tile_center_world_pos = Vec2::new(x_idx_i32 as f32 + 0.5, y_idx_i32 as f32 + 0.5);

            if (tile_center_world_pos - center_world_pos).length_squared() <= r_squared {
                if apply_on_tile(tile_x, tile_y, simulation) {
                    action_performed_overall = true;
                }
            }
        }
    }
    action_performed_overall
}
