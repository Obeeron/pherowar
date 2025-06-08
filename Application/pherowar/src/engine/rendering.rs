use super::GameCamera;
use crate::config::ASSETS_DIR;
use crate::simulation::{
    ANT_LENGTH, AntRef, COLONY_NEST_SIZE, Colony, DEFAULT_FOOD_AMOUNT, GameMap,
    MAX_PHEROMONE_AMOUNT, Simulation, Terrain,
};
use crate::ui::components::PheromoneDisplayMode;
use macroquad::prelude::*;

/// Enum representing possible camera actions like dragging or zooming.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CameraAction {
    /// Camera is being dragged.
    Drag,
    /// Camera is being zoomed.
    Zoom,
    /// No camera action is occurring.
    None,
}

/// Handles rendering of the game world, including map, ants, colonies, and UI elements.
pub struct Renderer {
    /// Texture for rendering ants.
    ant_texture: Texture2D,
    /// Texture for rendering food.
    food_texture: Texture2D,
    /// The main game camera.
    pub game_camera: GameCamera,
    /// Flag indicating if the camera is currently being dragged.
    is_dragging: bool,
    /// World position where the camera drag started.
    drag_start_world_pos: Vec2,
    /// Camera used for rendering the static map canvas.
    static_canvas_camera: Camera2D,
    /// Flag indicating if the static map canvas needs to be redrawn.
    is_wall_texture_dirty: bool,
}

impl Renderer {
    /// Creates a new `Renderer` instance.
    pub async fn new(map_width: u32, map_height: u32) -> Self {
        let camera = GameCamera::new(map_width, map_height);

        let ant_texture = load_texture(&format!("{}ant.png", ASSETS_DIR))
            .await
            .expect("Failed to load assets/ant.png");
        ant_texture.set_filter(FilterMode::Linear);

        let food_texture = load_texture(&format!("{}food.png", ASSETS_DIR))
            .await
            .expect("Failed to load assets/food.png");
        food_texture.set_filter(FilterMode::Linear);

        let canvas = render_target(map_width, map_height);
        canvas.texture.set_filter(FilterMode::Nearest);

        let mut static_canvas_camera =
            Camera2D::from_display_rect(Rect::new(0.0, 0.0, map_width as f32, map_height as f32));
        static_canvas_camera.render_target = Some(canvas);

        Self {
            ant_texture,
            food_texture,
            game_camera: camera,
            is_dragging: false,
            drag_start_world_pos: Vec2::ZERO,
            static_canvas_camera,
            is_wall_texture_dirty: true,
        }
    }

    /// Processes mouse wheel input for zooming the camera.
    pub fn process_mouse_wheel_zoom(&mut self) -> CameraAction {
        let wheel_movement = mouse_wheel().1;
        if wheel_movement != 0.0 {
            self.game_camera.adjust_zoom(-wheel_movement);
            return CameraAction::Zoom;
        }
        CameraAction::None
    }

    /// Processes mouse drag input for panning the camera.
    pub fn process_mouse_drag_pan(&mut self) -> CameraAction {
        let current_mouse_pos = Vec2::from(mouse_position());
        let mut drag_action_occurred = false;

        if is_mouse_button_pressed(MouseButton::Left) {
            self.is_dragging = true;
            self.drag_start_world_pos = self.game_camera.camera.screen_to_world(current_mouse_pos);
        }

        if self.is_dragging {
            if is_mouse_button_down(MouseButton::Left) {
                let current_world_pos = self.game_camera.camera.screen_to_world(current_mouse_pos);
                let world_offset_from_start = current_world_pos - self.drag_start_world_pos;

                const DRAG_MOVEMENT_THRESHOLD_SQ: f32 = 0.01;

                if world_offset_from_start.length_squared() > DRAG_MOVEMENT_THRESHOLD_SQ {
                    self.game_camera.move_by(-world_offset_from_start);
                    drag_action_occurred = true;
                }
            }

            if is_mouse_button_released(MouseButton::Left) {
                self.is_dragging = false;
            }
        }

        if drag_action_occurred {
            CameraAction::Drag
        } else {
            CameraAction::None
        }
    }

    /// Main rendering function, draws all game elements.
    pub fn render(
        &mut self,
        simulation: &Simulation,
        pheromone_mode: PheromoneDisplayMode,
        selected_ant_ref: Option<&AntRef>,
        show_ants: bool,
    ) {
        set_camera(&self.game_camera.camera);

        self.draw_map(&simulation.map);
        self.draw_pheromones(&simulation.colonies, pheromone_mode);
        self.draw_food(&simulation.map);
        if show_ants {
            self.draw_ants(simulation, selected_ant_ref);
        }
        self.draw_colonies(simulation);
    }

    /// Draws the static map elements (e.g., walls) to an offscreen canvas.
    fn draw_map(&mut self, map: &GameMap) {
        // Redraw static map if dirty
        if self.is_wall_texture_dirty {
            // Use the pre-configured static canvas camera
            let rt_camera = &self.static_canvas_camera;

            push_camera_state();
            set_camera(rt_camera);

            clear_background(Color::from_hex(0x222222));

            self.draw_walls(map);

            pop_camera_state();
            self.is_wall_texture_dirty = false;
        }

        let map_width = map.width as f32;
        let map_height = map.height as f32;

        // Draw the cached static canvas texture
        if let Some(render_target) = self.static_canvas_camera.render_target.as_ref() {
            let static_texture = &render_target.texture;
            draw_texture_ex(
                static_texture,
                0.0,
                map_height,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(Vec2::new(map_width, -map_height)),
                    ..Default::default()
                },
            );
        }
    }

    /// Draws food items on the map.
    fn draw_food(&self, map: &GameMap) {
        // Draw food textures dynamically each frame
        for y in 0..map.height as usize {
            for x in 0..map.width as usize {
                let pos_x = x as f32;
                let pos_y = y as f32;

                if let Some(Terrain::Food(amount)) = map.get_terrain_at(x, y) {
                    if *amount > 0 {
                        let intensity =
                            (*amount as f32 / DEFAULT_FOOD_AMOUNT as f32).clamp(0.0, 1.0);
                        draw_texture_ex(
                            &self.food_texture,
                            pos_x,
                            pos_y,
                            Color::new(1.0, 1.0, 1.0, 0.2 + intensity * 0.8),
                            DrawTextureParams {
                                dest_size: Some(Vec2::new(1.0, 1.0)),
                                ..Default::default()
                            },
                        );
                    }
                }
            }
        }
    }

    /// Draws pheromone trails on the map based on the selected display mode.
    fn draw_pheromones(
        &self,
        colonies: &std::collections::HashMap<u32, Colony>,
        pheromone_mode: PheromoneDisplayMode,
    ) {
        let channel_colors = super::CHANNEL_COLORS;
        match pheromone_mode {
            PheromoneDisplayMode::None => {}
            PheromoneDisplayMode::Colony { colony_id } => {
                if let Some(colony) = colonies.get(&colony_id) {
                    let base_color = colony.color;
                    let height = colony.pheromones[0].height as usize;
                    let width = colony.pheromones[0].width as usize;
                    for y in 0..height {
                        for x in 0..width {
                            let mut total = 0.0;
                            for channel in &colony.pheromones {
                                total += channel.data[y][x];
                            }
                            if total < 0.01 {
                                continue;
                            }
                            let alpha = (total / MAX_PHEROMONE_AMOUNT).clamp(0.0, 1.0);
                            draw_rectangle(
                                x as f32 + 0.2,
                                y as f32 + 0.2,
                                0.6,
                                0.6,
                                Color::new(base_color.r, base_color.g, base_color.b, alpha),
                            );
                        }
                    }
                }
            }
            PheromoneDisplayMode::Channel { colony_id, channel } => {
                let channel_idx = (channel as usize).saturating_sub(1);
                if let Some(colony) = colonies.get(&colony_id) {
                    let height = colony.pheromones[0].height as usize;
                    let width = colony.pheromones[0].width as usize;
                    if channel_idx < colony.pheromones.len() {
                        let channel_data = &colony.pheromones[channel_idx];
                        let base_tint = channel_colors[channel_idx % channel_colors.len()];
                        for y in 0..height {
                            for x in 0..width {
                                let val = channel_data.data[y][x];
                                if val < 0.01 {
                                    continue;
                                }
                                let intensity_ratio = (val / MAX_PHEROMONE_AMOUNT).clamp(0.0, 1.0);
                                // Threshold
                                // Sharper transition to white, more saturated base color
                                let color_interpolation_factor = intensity_ratio.powf(3.0); // Adjust exponent for desired curve
                                let r =
                                    base_tint.r + (1.0 - base_tint.r) * color_interpolation_factor;
                                let g =
                                    base_tint.g + (1.0 - base_tint.g) * color_interpolation_factor;
                                let b =
                                    base_tint.b + (1.0 - base_tint.b) * color_interpolation_factor;
                                draw_rectangle(
                                    x as f32,
                                    y as f32,
                                    1.0,
                                    1.0,
                                    Color::new(r, g, b, intensity_ratio), // Opacity still based on raw intensity_ratio
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    /// Draws wall tiles on the map with edge highlighting.
    fn draw_walls(&self, map: &GameMap) {
        let base_color_val = super::WALL_BASE_COLOR_VAL;
        let base_r = ((base_color_val >> 16) & 0xFF) as f32 / 255.0;
        let base_g = ((base_color_val >> 8) & 0xFF) as f32 / 255.0;
        let base_b = (base_color_val & 0xFF) as f32 / 255.0;

        let edge_brightness_boost = super::WALL_EDGE_BRIGHTNESS_BOOST;
        let edge_saturation_boost = super::WALL_EDGE_SATURATION_BOOST;

        for y in 0..map.height as usize {
            for x in 0..map.width as usize {
                if let Some(Terrain::Wall) = map.get_terrain_at(x, y) {
                    let pos_x = x as f32;
                    let pos_y = y as f32;

                    let brightness_variation = super::WALL_BRIGHTNESS_VARIATION;

                    // Calculate edge factor (0.0 to 1.0)
                    let mut num_non_wall_neighbors = 0;
                    let neighbors = [
                        (x.wrapping_sub(1), y),
                        (x + 1, y),
                        (x, y.wrapping_sub(1)),
                        (x, y + 1),
                        (x.wrapping_sub(1), y.wrapping_sub(1)),
                        (x + 1, y.wrapping_sub(1)),
                        (x.wrapping_sub(1), y + 1),
                        (x + 1, y + 1),
                        (x, y.wrapping_sub(2)),
                        (x, y + 2),
                        (x.wrapping_sub(2), y),
                        (x + 2, y),
                        (x.wrapping_sub(1), y.wrapping_sub(2)),
                        (x + 1, y.wrapping_sub(2)),
                        (x.wrapping_sub(2), y.wrapping_sub(1)),
                        (x + 2, y.wrapping_sub(1)),
                        (x.wrapping_sub(1), y + 2),
                        (x + 1, y + 2),
                        (x.wrapping_sub(2), y + 1),
                        (x + 2, y + 1),
                    ];

                    for (nx, ny) in neighbors {
                        if !matches!(map.get_terrain_at(nx, ny), Some(Terrain::Wall)) {
                            num_non_wall_neighbors += 1;
                        }
                    }
                    let edge_factor = (num_non_wall_neighbors as f32
                        / (neighbors.len() as f32 / 2.0))
                        .clamp(0.0, 1.0);

                    // Apply edge highlighting subtly
                    let final_r = (base_r * brightness_variation
                        + edge_brightness_boost * edge_factor)
                        .clamp(0.0, 1.0);
                    let final_g = (base_g * brightness_variation
                        + edge_brightness_boost * edge_factor)
                        .clamp(0.0, 1.0);
                    let final_b = (base_b * brightness_variation
                        + edge_brightness_boost * edge_factor)
                        .clamp(0.0, 1.0);

                    // Simple saturation boost approximation
                    let avg = (final_r + final_g + final_b) / 3.0;
                    let sat_r = final_r + (final_r - avg) * edge_saturation_boost * edge_factor;
                    let sat_g = final_g + (final_g - avg) * edge_saturation_boost * edge_factor;
                    let sat_b = final_b + (final_b - avg) * edge_saturation_boost * edge_factor;

                    let final_color = Color::new(
                        sat_r.clamp(0.0, 1.0),
                        sat_g.clamp(0.0, 1.0),
                        sat_b.clamp(0.0, 1.0),
                        1.0,
                    );

                    draw_rectangle(pos_x, pos_y, 1.0, 1.0, final_color);
                }
            }
        }
    }

    /// Draws ants on the map, highlighting the selected ant if any.
    fn draw_ants(&self, simulation: &Simulation, selected_ant_ref: Option<&AntRef>) {
        for (_colony_id_map, colony_obj) in &simulation.colonies {
            for (_ant_key_map, ant_obj) in &colony_obj.ants {
                let mut current_ant_color = colony_obj.color;
                if ant_obj.carrying_food {
                    current_ant_color.r = (current_ant_color.r + 0.2).min(1.0);
                    current_ant_color.g = (current_ant_color.g + 0.2).min(1.0);
                    current_ant_color.b = (current_ant_color.b + 0.2).min(1.0);
                }

                draw_texture_ex(
                    &self.ant_texture,
                    ant_obj.pos.x - ANT_LENGTH / 2.0,
                    ant_obj.pos.y - ANT_LENGTH / 2.0,
                    current_ant_color,
                    DrawTextureParams {
                        dest_size: Some(Vec2::new(ANT_LENGTH, ANT_LENGTH)),
                        rotation: ant_obj.rotation,
                        ..Default::default()
                    },
                );

                if let Some(selected_ref) = selected_ant_ref {
                    if selected_ref == &ant_obj.ant_ref {
                        let highlight_radius = ANT_LENGTH * 0.7;
                        let highlight_color = Color::new(1.0, 0.9, 0.2, 0.9);
                        let line_thickness = ANT_LENGTH * 0.15;
                        draw_circle_lines(
                            ant_obj.pos.x,
                            ant_obj.pos.y,
                            highlight_radius,
                            line_thickness,
                            highlight_color,
                        );
                    }
                }
            }
        }
    }

    /// Draws colony nests and placeholder colony locations.
    fn draw_colonies(&self, simulation: &Simulation) {
        for (_, colony) in &simulation.colonies {
            let is_dead = colony.is_dead();
            let colony_color = if is_dead { BLACK } else { colony.color };
            let outline_color = Color::new(
                colony_color.r * 0.5,
                colony_color.g * 0.5,
                colony_color.b * 0.5,
                1.0,
            );

            // Draw colony base and outline
            draw_circle(
                colony.pos.x,
                colony.pos.y,
                COLONY_NEST_SIZE / 2.0,
                colony_color,
            );
            draw_circle_lines(
                colony.pos.x,
                colony.pos.y,
                COLONY_NEST_SIZE / 2.0,
                0.2,
                outline_color,
            );

            // Draw skull emoji if dead
            if is_dead {
                let font_size = COLONY_NEST_SIZE * 1.2;
                let text = "x";
                let text_dim = measure_text(text, None, font_size as u16, 1.0);
                draw_text(
                    text,
                    colony.pos.x - text_dim.width / 2.0,
                    colony.pos.y + text_dim.height / 2.0,
                    font_size,
                    colony.color,
                );
            }
        }

        // Draw placeholder colonies
        let placeholder_color = GRAY;
        let placeholder_outline_color = DARKGRAY;
        // Iterate over simulation.map.placeholder_colony_locations directly
        for pos in &simulation.map.placeholder_colony_locations {
            draw_circle(pos.x, pos.y, COLONY_NEST_SIZE / 2.0, placeholder_color);
            draw_circle_lines(
                pos.x,
                pos.y,
                COLONY_NEST_SIZE / 2.0,
                0.2,
                placeholder_outline_color,
            );
        }
    }

    /// Resets the renderer state, typically when the map changes.
    pub fn reset(&mut self, width: u32, height: u32) {
        self.game_camera.reset();
        self.update_map_size(width, height);
        self.is_dragging = false;
    }

    /// Updates the renderer and camera settings for a new map size.
    fn update_map_size(&mut self, width: u32, height: u32) {
        // Update GameCamera dimensions
        self.game_camera.map_width = width;
        self.game_camera.map_height = height;
        self.game_camera.reset(); // Reset zoom/position based on new dimensions

        // Recreate render target for the static canvas
        let canvas = render_target(width, height);
        canvas.texture.set_filter(FilterMode::Nearest);

        // Reconfigure the static canvas camera
        let mut static_canvas_camera =
            Camera2D::from_display_rect(Rect::new(0.0, 0.0, width as f32, height as f32));
        static_canvas_camera.render_target = Some(canvas);
        self.static_canvas_camera = static_canvas_camera;

        // Mark the renderer as dirty to redraw the static parts
        self.mark_dirty();
    }

    /// Marks the static map canvas as dirty, forcing a redraw on the next frame.
    pub fn mark_dirty(&mut self) {
        self.is_wall_texture_dirty = true;
    }
}
