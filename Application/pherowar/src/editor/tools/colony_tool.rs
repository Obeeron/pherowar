use crate::config::PlayerConfig;
use crate::editor::color_palette::ColorPalette;
use crate::simulation::{COLONY_NEST_SIZE, Simulation};
use macroquad::prelude::{Color, IVec2, Vec2, WHITE};

/// Converts world position (Vec2) to integer tile coordinates (IVec2).
fn world_pos_to_tile_coord(world_pos: Vec2) -> IVec2 {
    IVec2::new(world_pos.x.floor() as i32, world_pos.y.floor() as i32)
}

/// Converts integer tile coordinates (IVec2) to world center position (Vec2, e.g., X.5, Y.5).
fn tile_coord_to_world_center(tile_coord: IVec2) -> Vec2 {
    Vec2::new(tile_coord.x as f32 + 0.5, tile_coord.y as f32 + 0.5)
}

/// Determines effective target tile: snaps to an existing entity's center if click is within its footprint.
fn determine_effective_target_tile(clicked_tile: IVec2, simulation: &Simulation) -> IVec2 {
    let entity_half_size = (COLONY_NEST_SIZE / 2.0).floor() as i32;

    // Check colonies
    for colony in simulation.colonies.values() {
        let colony_center_tile = world_pos_to_tile_coord(colony.pos);
        let min_x = colony_center_tile.x - entity_half_size;
        let max_x = colony_center_tile.x + entity_half_size;
        let min_y = colony_center_tile.y - entity_half_size;
        let max_y = colony_center_tile.y + entity_half_size;

        if clicked_tile.x >= min_x
            && clicked_tile.x <= max_x
            && clicked_tile.y >= min_y
            && clicked_tile.y <= max_y
        {
            return colony_center_tile; // Snap to this colony's center
        }
    }

    // Check placeholders
    for p_center_world_pos in &simulation.map.placeholder_colony_locations {
        let placeholder_center_tile = world_pos_to_tile_coord(*p_center_world_pos);
        let min_x = placeholder_center_tile.x - entity_half_size;
        let max_x = placeholder_center_tile.x + entity_half_size;
        let min_y = placeholder_center_tile.y - entity_half_size;
        let max_y = placeholder_center_tile.y + entity_half_size;

        if clicked_tile.x >= min_x
            && clicked_tile.x <= max_x
            && clicked_tile.y >= min_y
            && clicked_tile.y <= max_y
        {
            return placeholder_center_tile; // Snap to this placeholder's center
        }
    }

    clicked_tile // No snap, use original clicked tile
}

/// Removes colony or placeholder centered at `target_tile_coord`. Returns true if removed.
fn handle_remove_entity_at_tile(simulation: &mut Simulation, target_tile_coord: IVec2) -> bool {
    let mut removed_any = false;

    // Remove colonies centered on the target tile
    let mut colonies_to_remove_ids = Vec::new();
    for (id, colony) in &simulation.colonies {
        if world_pos_to_tile_coord(colony.pos) == target_tile_coord {
            colonies_to_remove_ids.push(*id);
        }
    }
    for id in colonies_to_remove_ids {
        if simulation.remove_colony(id) {
            removed_any = true;
        }
    }

    // Remove placeholder centered on the target tile
    let mut placeholder_found_on_tile = false;
    for p_center_pos in &simulation.map.placeholder_colony_locations {
        if world_pos_to_tile_coord(*p_center_pos) == target_tile_coord {
            placeholder_found_on_tile = true;
            break;
        }
    }

    if placeholder_found_on_tile {
        // `remove_placeholder_colony` expects the tile's (X.0, Y.0) Vec2 coordinate.
        let tile_snapped_coord_for_removal =
            Vec2::new(target_tile_coord.x as f32, target_tile_coord.y as f32);
        if simulation
            .map
            .remove_placeholder_colony(tile_snapped_coord_for_removal)
        {
            removed_any = true;
        }
    }
    removed_any
}

/// Clears `target_tile_coord` by removing any colony/placeholder centered there. Returns true if removed.
fn clear_tile_for_new_entity(target_tile_coord: IVec2, simulation: &mut Simulation) -> bool {
    // Currently identical to handle_remove_entity_at_tile.
    handle_remove_entity_at_tile(simulation, target_tile_coord)
}

/// Checks if placing a new entity (5x5 area) at `target_center_tile` would overlap with OTHERS.
fn is_placement_area_valid(target_center_tile: IVec2, simulation: &Simulation) -> bool {
    let entity_half_size = (COLONY_NEST_SIZE / 2.0).floor() as i32;

    // Bounding box of the new entity
    let new_min_x = target_center_tile.x - entity_half_size;
    let new_max_x = target_center_tile.x + entity_half_size;
    let new_min_y = target_center_tile.y - entity_half_size;
    let new_max_y = target_center_tile.y + entity_half_size;

    // Check against other colonies
    for colony in simulation.colonies.values() {
        let existing_center_tile = world_pos_to_tile_coord(colony.pos);
        if existing_center_tile == target_center_tile {
            continue;
        } // Skip self (already cleared)

        let existing_min_x = existing_center_tile.x - entity_half_size;
        let existing_max_x = existing_center_tile.x + entity_half_size;
        let existing_min_y = existing_center_tile.y - entity_half_size;
        let existing_max_y = existing_center_tile.y + entity_half_size;

        // AABB collision check
        if new_min_x <= existing_max_x
            && new_max_x >= existing_min_x
            && new_min_y <= existing_max_y
            && new_max_y >= existing_min_y
        {
            eprintln!(
                "[WARN] Proximity (Tile): Too close to colony at {:?}. Target: {:?}. Size: {}",
                existing_center_tile, target_center_tile, COLONY_NEST_SIZE
            );
            return false;
        }
    }

    // Check against other placeholders
    for p_center_pos in &simulation.map.placeholder_colony_locations {
        let existing_center_tile = world_pos_to_tile_coord(*p_center_pos);
        if existing_center_tile == target_center_tile {
            continue;
        } // Skip self

        let existing_min_x = existing_center_tile.x - entity_half_size;
        let existing_max_x = existing_center_tile.x + entity_half_size;
        let existing_min_y = existing_center_tile.y - entity_half_size;
        let existing_max_y = existing_center_tile.y + entity_half_size;

        if new_min_x <= existing_max_x
            && new_max_x >= existing_min_x
            && new_min_y <= existing_max_y
            && new_max_y >= existing_min_y
        {
            eprintln!(
                "[WARN] Proximity (Tile): Too close to placeholder at {:?}. Target: {:?}. Size: {}",
                existing_center_tile, target_center_tile, COLONY_NEST_SIZE
            );
            return false;
        }
    }
    true
}

/// Resolves final color for a new colony, finding next available if initial is used.
fn resolve_final_colony_color(
    initial_color: Color,
    simulation: &Simulation,
    color_palette: &mut ColorPalette,
) -> Option<Color> {
    if !ColorPalette::is_color_used(initial_color, simulation)
        || ColorPalette::are_all_colors_used(simulation)
    // If all used, allow re-using current selection
    {
        Some(initial_color)
    } else {
        // Initial color is used, and not all colors are exhausted; try to find a new one.
        if color_palette.update_selection(simulation) {
            let new_selected_color = color_palette.get_selected_color();
            // Check if the *newly selected* color is usable (either not used, or all are used)
            if !ColorPalette::is_color_used(new_selected_color, simulation)
                || ColorPalette::are_all_colors_used(simulation)
            {
                Some(new_selected_color)
            } else {
                eprintln!(
                    "[WARN] Color resolve: Palette updated, but new color {:?} still used (and not all exhausted).",
                    new_selected_color
                );
                None // Should ideally not happen if update_selection works correctly
            }
        } else {
            eprintln!(
                "[WARN] Color resolve: Failed to update palette. All colors might be used or no players."
            );
            None // Cannot find an alternative color
        }
    }
}

/// Applies the colony tool: places or removes colonies/placeholders.
pub fn apply_colony(
    raw_world_pos: Vec2,
    is_removing: bool,
    current_player_index: Option<usize>,
    player_configs: &Vec<PlayerConfig>,
    color_palette: &mut ColorPalette,
    simulation: &mut Simulation,
) -> bool {
    let initial_clicked_tile_coord = world_pos_to_tile_coord(raw_world_pos);
    // Determine the actual entity or tile being targeted by snapping to footprint if necessary.
    let effective_target_tile =
        determine_effective_target_tile(initial_clicked_tile_coord, simulation);

    if is_removing {
        // Remove entity at the (potentially snapped) target tile.
        return handle_remove_entity_at_tile(simulation, effective_target_tile);
    } else {
        // Placement Logic
        let mut change_occurred_before_placement = false;

        // 1. Clear the target spot (center tile of the new/targeted entity).
        if clear_tile_for_new_entity(effective_target_tile, simulation) {
            change_occurred_before_placement = true;
        }

        // 2. Validate Position: check 5x5 area overlap with *other* entities.
        if !is_placement_area_valid(effective_target_tile, simulation) {
            eprintln!(
                "[WARN] Placement failed: Area for tile {:?} overlaps existing entity.",
                effective_target_tile
            );
            return change_occurred_before_placement; // Return if clearing did anything
        }

        // 3. Execute Placement
        let target_world_center_pos = tile_coord_to_world_center(effective_target_tile);
        // `target_cell_snapped_coord_vec2` is for map.add_placeholder_colony which expects (X.0, Y.0)

        match current_player_index {
            Some(0) => {
                // Place Placeholder
                let cell_x_usize = effective_target_tile.x as usize;
                let cell_y_usize = effective_target_tile.y as usize;

                if simulation.place_nest_placeholder_at(cell_x_usize, cell_y_usize) {
                    return true;
                }
                eprintln!(
                    "[WARN] Placeholder add failed at {:?}. Tile might be occupied or out of bounds.",
                    effective_target_tile
                );
                return change_occurred_before_placement;
            }
            Some(player_idx_1_based) => {
                // Place Player Colony
                if player_idx_1_based == 0 {
                    // Should be caught by Some(0) case above
                    eprintln!("[ERROR] Invalid player_idx 0 for Player Colony.");
                    return change_occurred_before_placement;
                }
                let player_config_index = player_idx_1_based - 1;

                if let Some(player_cfg) = player_configs.get(player_config_index) {
                    let initial_color = color_palette.get_selected_color();
                    let final_color = match resolve_final_colony_color(
                        initial_color,
                        simulation,
                        color_palette,
                    ) {
                        Some(c) => c,
                        None => {
                            eprintln!(
                                "[WARN] Colony color resolution failed for player {}.",
                                player_idx_1_based
                            );
                            return change_occurred_before_placement;
                        }
                    };

                    simulation.spawn_colony(
                        target_world_center_pos,
                        final_color,
                        player_cfg.clone(),
                    );
                    color_palette.update_selection(simulation); // Advance to next available color
                    return true;
                }

                eprintln!("[WARN] No player config for index: {}", player_idx_1_based);
                return change_occurred_before_placement;
            }
            None => {
                // No player or placeholder selected
                eprintln!("[INFO] No player/placeholder selected for placement.");
                return change_occurred_before_placement;
            }
        }
    }
}

/// Renders the preview for the colony tool.
pub fn render_colony_preview(
    world_pos: Vec2,
    is_removing: bool,
    current_player_index: Option<usize>,
) {
    let radius = COLONY_NEST_SIZE / 2.0;
    // Preview follows mouse cursor directly, not snapped.
    let preview_center_x = world_pos.x;
    let preview_center_y = world_pos.y;

    let color = if is_removing {
        Color::new(1.0, 0.2, 0.2, 0.5) // Reddish for removal
    } else {
        match current_player_index {
            Some(0) => Color::new(0.7, 0.7, 1.0, 0.5), // Bluish for placeholder
            Some(_) => Color::new(0.2, 1.0, 0.2, 0.5), // Greenish for player colony
            None => Color::new(0.5, 0.5, 0.5, 0.3),    // Dim if no selection
        }
    };
    macroquad::shapes::draw_circle(preview_center_x, preview_center_y, radius, color);
    macroquad::shapes::draw_circle_lines(preview_center_x, preview_center_y, radius, 0.4, WHITE);
}

/// Colony tool is not draggable (single click placement/removal).
pub fn is_colony_tool_draggable() -> bool {
    false
}
