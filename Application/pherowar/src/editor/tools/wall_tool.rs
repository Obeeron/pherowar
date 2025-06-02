use crate::editor::tools::helpers::apply_action_in_circular_area;
use crate::simulation::{Simulation, Terrain};
use macroquad::prelude::{Color, Vec2, WHITE};

// WallTool specific logic

pub fn apply_wall(
    world_pos: Vec2,
    tool_size: f32,
    is_removing: bool,
    simulation: &mut Simulation,
) -> bool {
    apply_action_in_circular_area(world_pos, tool_size, simulation, |tile_x, tile_y, sim| {
        if is_removing {
            if let Some(Terrain::Wall) = sim.get_terrain_at(tile_x, tile_y) {
                sim.remove_terrain_at(tile_x, tile_y);
                true
            } else {
                false
            }
        } else {
            if let Some(Terrain::Empty) = sim.get_terrain_at(tile_x, tile_y) {
                sim.place_wall_at(tile_x, tile_y);
                true
            } else {
                false
            }
        }
    })
}

pub fn render_wall_preview(world_pos: Vec2, tool_size: f32, is_removing: bool) {
    let color = if is_removing {
        Color::new(0.8, 0.8, 0.8, 0.5)
    } else {
        Color::new(0.5, 0.5, 0.5, 0.5)
    };
    macroquad::shapes::draw_circle(world_pos.x, world_pos.y, tool_size / 2.0, color);
    macroquad::shapes::draw_circle_lines(world_pos.x, world_pos.y, tool_size / 2.0, 0.4, WHITE);
}

pub fn is_wall_tool_draggable() -> bool {
    true
}
