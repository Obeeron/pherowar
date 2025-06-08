use crate::config::AppConfig;
use crate::editor::{EditorManager, ToolType};
use crate::engine::{CameraAction, Renderer};
use crate::simulation::{GameMap, Simulation, THINK_INTERVAL};
use crate::ui::UIManager;
use crate::ui::components::DialogPopup;
use crate::ui::events::AppAction;
use macroquad::prelude::*;
use std::cell::RefCell;
use std::time::Instant;

thread_local! {
    static LAST_DOUBLE_CLICK_INFO: RefCell<Option<(Instant, (f32, f32))>> = RefCell::new(None);
}

/// Main application structure for PheroWar.
pub struct PWApp {
    ui: UIManager,          // Manages all UI elements and interactions.
    editor: EditorManager,  // Handles map editing tools and state.
    renderer: Renderer,     // Responsible for drawing the game world and UI.
    simulation: Simulation, // Core game logic, including ants, colonies, and map state.
    winner_announced: bool, // Flag to ensure the winner announcement dialog is shown only once.
}

impl PWApp {
    /// Creates a new `PWApp` instance.
    pub async fn new(app_config: AppConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let player_configs = app_config.player_configs;

        let simulation = if let Some(map_name) = &app_config.simulation.map {
            let loaded_map = crate::simulation::GameMap::load_map_with_dir(
                map_name,
                app_config.simulation.maps_dir.as_deref(),
            )?;

            // Validate player count if CLI players are provided
            if let Some(ref players) = app_config.cli_players {
                let expected_colonies = loaded_map.placeholder_colony_locations.len();
                let provided_players = players.len();

                if expected_colonies != provided_players {
                    return Err(format!(
                        "Colony count mismatch: Map '{}' expects {} colonies but {} players were provided",
                        map_name, expected_colonies, provided_players
                    ).into());
                }
            }

            // Create simulation with the loaded map
            let mut sim = Simulation::new(&app_config.simulation, player_configs.clone(), None);
            sim.map = loaded_map;
            sim
        } else {
            Simulation::new(&app_config.simulation, player_configs.clone(), None)
        };

        let renderer = Renderer::new(simulation.map.width, simulation.map.height).await;

        let mut app = Self {
            ui: UIManager::new(),
            editor: EditorManager::new(&simulation.player_configs),
            renderer,
            simulation,
            winner_announced: false,
        };

        // Auto-spawn colonies if CLI players were provided
        if let Some(players) = app_config.cli_players {
            let placeholder_locations = app.simulation.map.placeholder_colony_locations.clone();

            for (i, player_name) in players.iter().enumerate() {
                let player_cfg = player_configs
                    .iter()
                    .find(|p| p.name == *player_name)
                    .ok_or_else(|| format!("Player config for '{}' not found", player_name))?
                    .clone();

                let pos = placeholder_locations[i];

                let color = crate::editor::color_palette::PREDEFINED_COLONY_COLORS
                    [i % crate::editor::color_palette::PREDEFINED_COLONY_COLORS.len()];

                app.simulation.spawn_colony(pos, color, player_cfg);
            }
        }

        Ok(app)
    }

    /// Runs the main application loop.
    pub async fn run(&mut self) {
        let mut last_time = get_time(); // wall-clock seconds

        loop {
            let frame_start = get_time();
            // Measure real elapsed time since last frame
            let now = get_time();
            let dt = now - last_time;
            last_time = now;

            if self.ui.unlimited() {
                // Dynamically adjust max_dt based on ant count
                let ant_count = self.simulation.total_ant_count();
                let max_dt = (THINK_INTERVAL / (ant_count as f32 / 1000.0)).min(THINK_INTERVAL);
                // Run as many simulation steps as possible until it's time to render
                let target_frame_time = 1.0 / 60.0; // 60 FPS
                while get_time() - frame_start < target_frame_time {
                    self.simulation.update(max_dt);
                }
            } else {
                let time_multiplier = self.ui.time_multiplier().unwrap_or(1.0);
                let mut sim_dt = (dt as f32) * time_multiplier;
                while sim_dt > 0.0 {
                    let step = sim_dt.min(THINK_INTERVAL);
                    self.simulation.update(step);
                    sim_dt -= step;
                }
            }

            if self.simulation.colonies.len() > 1 {
                self.check_winner();
            }

            // Draw one frame
            self.update_ui();
            self.render();

            // Yield back to Macroquad (swap buffers, poll events, vsync)
            next_frame().await;
        }
    }

    /// Checks if a winner has emerged in the simulation.
    fn check_winner(&mut self) {
        // Check if a single colony remains
        let alive_keys: Vec<_> = self
            .simulation
            .colonies
            .iter()
            .filter(|(_, c)| !c.is_dead())
            .map(|(k, _)| k.clone())
            .collect();

        if alive_keys.len() == 1 && !self.winner_announced {
            self.simulation.pause();
            let winner_name = &self.simulation.colonies[&alive_keys[0]].player_config.name;
            // Only show dialog if not already open
            if self.ui.dialog_popup.is_none() && !self.winner_announced {
                self.winner_announced = true;
                self.ui
                    .show_dialog(crate::ui::components::DialogPopup::new_info(&format!(
                        "🏆 {} wins! 🏆\nGreat antgineering.",
                        winner_name
                    )));
            }
        } else if alive_keys.len() >= 2 {
            // Reset winner announcement flag if there are multiple colonies alive
            self.winner_announced = false;
        }
    }

    /// Updates the UI state and handles input.
    fn update_ui(&mut self) {
        // Handle global shortcuts first, as they might trigger actions
        let shortcut_handled = self.handle_global_shortcuts();

        // UIManager now handles selected_ant_data and is_camera_locked internally.
        let (app_action, ui_consumed_input) = self.ui.update(
            &mut self.editor,
            &self.simulation, // Pass simulation for UIManager to get ant data
            &mut self.renderer.game_camera,
        );

        // Handle actions generated by UI or shortcuts
        self.handle_app_actions(app_action);

        // Handle world input if not consumed by UI or shortcuts
        if !shortcut_handled && !ui_consumed_input {
            self.handle_world_input();
        }

        // Handle camera lock and ant death using UIManager state
        if self.ui.is_camera_locked() {
            if let Some(locked_ant_ref) = self.ui.get_camera_locked_ant_ref() {
                if let Some(ant) = self.simulation.get_ant(locked_ant_ref) {
                    self.renderer.game_camera.set_target(ant.pos);
                } else {
                    // Ant died or is no longer available
                    // Pass the key of the locked ant for UIManager to handle
                    self.ui.handle_dead_ant(locked_ant_ref.key);
                }
            }
        }
    }

    /// Handles mouse and keyboard input related to the game world.
    fn handle_world_input(&mut self) {
        // Ant selection (ALT + Click or Double Left Click)
        const DOUBLE_CLICK_MAX_MS: u128 = 350;
        const DOUBLE_CLICK_MAX_DIST: f32 = 8.0;
        const DOUBLE_CLICK_MAX_DIST_SQ: f32 = DOUBLE_CLICK_MAX_DIST * DOUBLE_CLICK_MAX_DIST;

        let alt_down = is_key_down(KeyCode::LeftAlt) || is_key_down(KeyCode::RightAlt);
        let left_pressed = is_mouse_button_pressed(MouseButton::Left);
        let left_released = is_mouse_button_released(MouseButton::Left);
        let mouse_pos = mouse_position(); // screen coordinates
        let world_pos = self
            .renderer
            .game_camera
            .camera
            .screen_to_world(mouse_pos.into());

        let mut double_clicked = false;
        if left_released {
            let current_click_time = Instant::now();
            let current_mouse_pos = mouse_pos;

            LAST_DOUBLE_CLICK_INFO.with(|cell| {
                let mut opt_last_click = cell.borrow_mut();
                if let Some((last_time, last_pos)) = *opt_last_click {
                    let elapsed_ms = current_click_time.duration_since(last_time).as_millis();
                    let dx = current_mouse_pos.0 - last_pos.0;
                    let dy = current_mouse_pos.1 - last_pos.1;
                    let dist_sq = dx * dx + dy * dy;

                    if elapsed_ms < DOUBLE_CLICK_MAX_MS && dist_sq < DOUBLE_CLICK_MAX_DIST_SQ {
                        double_clicked = true;
                        // Reset last click info to prevent this click from immediately forming part of a new double-click
                        *opt_last_click = None;
                    } else {
                        // Not a double click, so this click becomes the new reference point
                        *opt_last_click = Some((current_click_time, current_mouse_pos));
                    }
                } else {
                    // No previous click stored, so store the current one
                    *opt_last_click = Some((current_click_time, current_mouse_pos));
                }
            });
        }

        if (alt_down && left_pressed) || double_clicked {
            if let Some(ant_ref) = self.simulation.get_ant_at_world_pos(world_pos, 5.0) {
                if self.ui.get_selected_ant_ref().map(|r| r.key) == Some(ant_ref.key) {
                    self.ui.deselect_ant();
                } else {
                    self.ui.select_ant(Some(ant_ref));
                }
            } else {
                self.ui.deselect_ant();
            }
            return; // Input consumed by ant selection/deselection
        }

        let ctrl_pressed = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
        let mouse_wheel_delta = mouse_wheel().1;
        let mut camera_dragged_this_frame = false;

        let world_pos_for_editor = self
            .renderer
            .game_camera
            .camera
            .screen_to_world(mouse_position().into());

        if self.editor.current_tool().is_some() {
            if ctrl_pressed {
                if mouse_wheel_delta != 0.0 {
                    // CTRL + Wheel: Tool resize. Editor handles this. Camera does not zoom here.
                    self.editor.handle_input(
                        &mut self.simulation,
                        &mut self.renderer,
                        world_pos_for_editor,
                    );
                } else {
                    // CTRL + Drag/Click: Camera pan. Renderer handles this.
                    if self.renderer.process_mouse_drag_pan() == CameraAction::Drag {
                        camera_dragged_this_frame = true;
                    }
                }
            } else {
                // Normal camera zoom (if wheel moved)
                if mouse_wheel_delta != 0.0 {
                    // We call process_mouse_wheel_zoom for its side effect (zooming).
                    // The return value CameraAction::Zoom or CameraAction::None isn't directly used here
                    // for camera_dragged_this_frame logic, but it correctly performs the zoom.
                    self.renderer.process_mouse_wheel_zoom();
                }
                // Normal tool usage (clicks/drags for painting, etc.)
                self.editor.handle_input(
                    &mut self.simulation,
                    &mut self.renderer,
                    world_pos_for_editor,
                );
            }
        } else {
            // Normal camera zoom (if wheel moved)
            if mouse_wheel_delta != 0.0 {
                self.renderer.process_mouse_wheel_zoom();
            }
            // Normal camera pan
            if self.renderer.process_mouse_drag_pan() == CameraAction::Drag {
                camera_dragged_this_frame = true;
            }
        }

        // Unlock camera if it was locked and a drag occurred this frame
        if camera_dragged_this_frame && self.ui.is_camera_locked() {
            self.ui.unlock_camera();
        }
    }

    /// Handles global keyboard shortcuts.
    fn handle_global_shortcuts(&mut self) -> bool {
        // If a dialog popup is open, do not process shortcuts
        if self.ui.dialog_popup.is_some() {
            return false;
        }

        // Tool selection shortcuts
        if is_key_pressed(KeyCode::Escape) {
            self.editor.set_tool(None);
            self.ui.deselect_ant(); // Use UIManager
            return true;
        } else if is_key_pressed(KeyCode::Key1) {
            self.editor.set_tool(Some(ToolType::Food));
            return true;
        } else if is_key_pressed(KeyCode::Key2) {
            self.editor.set_tool(Some(ToolType::Wall));
            return true;
        } else if is_key_pressed(KeyCode::Key3) {
            self.editor.set_tool(Some(ToolType::Colony));
            return true;
        }
        // Simulation control shortcuts
        else if is_key_pressed(KeyCode::P) || is_key_pressed(KeyCode::Space) {
            self.handle_app_actions(Some(AppAction::TogglePause));
            return true;
        } else if is_key_pressed(KeyCode::R) {
            self.handle_app_actions(Some(AppAction::RequestReset));
            return true;
        } else if is_key_pressed(KeyCode::S) {
            self.handle_app_actions(Some(AppAction::RequestSaveMap(String::new())));
            return true;
        } else if is_key_pressed(KeyCode::L) {
            self.handle_app_actions(Some(AppAction::RequestLoadMap(String::new())));
            return true;
        }
        // Toggle UI visibility shortcut
        if is_key_pressed(KeyCode::F) {
            self.ui.toggle_top_panel();
            return true;
        }
        // Toggle debug panel shortcut
        if is_key_pressed(KeyCode::D) {
            self.ui.toggle_debug_panel();
            return true;
        }
        // Toggle visual options panel shortcut
        if is_key_pressed(KeyCode::V) {
            self.ui.toggle_visual_options_panel();
            return true;
        }

        false
    }

    /// Processes application-level actions triggered by UI or shortcuts.
    fn handle_app_actions(&mut self, action: Option<AppAction>) {
        if let Some(action) = action {
            match action {
                AppAction::TogglePause => match self.simulation.try_toggle_pause() {
                    Ok(()) => {}
                    Err(msg) => {
                        self.ui.show_dialog(DialogPopup::new_info(&msg));
                    }
                },
                AppAction::RequestReset => {
                    self.reset();
                }
                AppAction::RequestSaveMap(name) => {
                    self.handle_save_map_request(name);
                }
                AppAction::RequestLoadMap(name) => {
                    self.handle_load_map_request(name);
                }
                AppAction::RequestNewMap { width, height } => {
                    self.simulation.create_new_map(width, height);
                    self.renderer.reset(width, height);
                    self.editor = EditorManager::new(&self.simulation.player_configs);
                }
                AppAction::ToggleCameraLockOnSelectedAnt => {
                    self.ui.toggle_camera_lock();
                }
            }
        }
    }

    /// Handles the request to save the current map.
    fn handle_save_map_request(&mut self, name: String) {
        let maps_dir = self.simulation.config.maps_dir.as_deref();
        if name.is_empty() {
            let prefill_name = self
                .simulation
                .map
                .loaded_map_name
                .clone()
                .unwrap_or_else(|| "Untitled.map".to_string());
            self.ui.show_dialog(DialogPopup::new_input(
                "Enter map name to save:",
                &prefill_name,
            ));
        } else {
            let res = self.simulation.map.save_map_with_dir(&name, maps_dir);
            if let Err(e) = res {
                self.ui
                    .show_dialog(DialogPopup::new_info(&format!("Failed to save map: {}", e)));
            } else {
                self.ui
                    .show_dialog(DialogPopup::new_info("Map saved successfully."));
            }
        }
    }

    /// Handles the request to load a map from file.
    fn handle_load_map_request(&mut self, name: String) {
        let maps_dir = self.simulation.config.maps_dir.as_deref();
        if name.is_empty() {
            match GameMap::list_maps_with_dir(maps_dir) {
                Ok(map_list) if !map_list.is_empty() => {
                    self.ui.show_dialog(DialogPopup::new_map_picker(map_list));
                }
                Ok(_) => {
                    self.ui
                        .show_dialog(DialogPopup::new_info("No maps found in maps/ directory."));
                }
                Err(e) => {
                    self.ui.show_dialog(DialogPopup::new_info(&format!(
                        "Failed to list maps: {}",
                        e
                    )));
                }
            }
        } else {
            match GameMap::load_map_with_dir(&name, maps_dir) {
                Ok(new_game_map) => {
                    let width = new_game_map.width;
                    let height = new_game_map.height;
                    self.simulation.map = new_game_map;
                    self.simulation.colonies.clear();
                    self.renderer.reset(width, height);
                    self.editor.color_palette.update_selection(&self.simulation);
                    self.ui.show_dialog(DialogPopup::new_info("Map loaded."));
                }
                Err(e) => {
                    self.ui
                        .show_dialog(DialogPopup::new_info(&format!("Failed to load map: {}", e)));
                }
            }
        }
    }

    /// Renders the current game state and UI.
    fn render(&mut self) {
        // Set the background color and camera for rendering game
        clear_background(Color::from_hex(0x181820));
        set_camera(&self.renderer.game_camera.camera);

        let pheromone_mode = self.ui.pheromone_display_mode();
        let show_ants = self.ui.show_ants(); // Get ant visibility state

        // Get selected ant *reference* via UIManager for rendering highlight
        let selected_ant_ref_for_render = self.ui.get_selected_ant_ref();

        self.renderer.render(
            &self.simulation,
            pheromone_mode,
            selected_ant_ref_for_render,
            show_ants,
        );

        // Render tool preview with the same camera if a tool is selected
        if self.editor.current_tool().is_some() {
            // Get world position directly from the camera
            let world_pos = self.renderer.game_camera.get_mouse_world_pos();
            self.editor.render_tool_preview(world_pos);
        }

        // Switch to default camera for UI rendering
        set_default_camera();

        // Render UI
        self.ui.render();
    }

    /// Resets the application to its initial state with the current map.
    fn reset(&mut self) {
        self.simulation.reset();
        self.editor = EditorManager::new(&self.simulation.player_configs);
        self.renderer
            .reset(self.simulation.map.width, self.simulation.map.height);
        self.editor.color_palette.update_selection(&self.simulation);
    }
}
