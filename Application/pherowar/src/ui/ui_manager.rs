use catppuccin_egui::set_theme;
use egui::{self};
use macroquad::prelude::*;

use crate::editor::EditorManager;
use crate::engine::GameCamera;
use crate::simulation::ant::{Ant, AntRef};
use crate::simulation::{DEFAULT_MAP_HEIGHT, DEFAULT_MAP_WIDTH, Simulation};
use crate::ui::components::{
    AntStatusBar, DebugPanel, DialogPopup, DialogPopupMode, DialogPopupResult, PheromoneDisplayMode, TopPanel,
    VisualOptionsPanel,
};
use crate::ui::events::{AppAction, UIEvent};

fn auto_zoom(ctx: &egui::Context, base_px: egui::Vec2) -> f32 {
    let logical = ctx.screen_rect().size();
    let win_px = logical * ctx.pixels_per_point(); // physical pixels
    let s = (win_px.x / base_px.x).min(win_px.y / base_px.y);
    ((s * 4.0).round() / 4.0).clamp(0.75, 3.0) // snap to 0.25 for crisp text
}

pub struct UIManager {
    drag_started_on_ui: bool,
    debug_panel: DebugPanel,
    pub top_panel: TopPanel,
    pub visual_options_panel: VisualOptionsPanel,
    pub ant_status_bar: AntStatusBar,
    pub dialog_popup: Option<DialogPopup>,
    selected_ant: Option<AntRef>,
    camera_locked_on_ant: Option<AntRef>,
    last_screen_size: (f32, f32), // Only for camera resize events
    last_win_px: egui::Vec2,
    top_panel_visible: bool,
}

impl UIManager {
    pub fn new() -> Self {
        let window_w = screen_width();
        let window_h = screen_height();
        Self {
            drag_started_on_ui: false,
            debug_panel: DebugPanel::new(),
            top_panel: TopPanel::new(),
            last_screen_size: (window_w, window_h),
            last_win_px: egui::vec2(0.0, 0.0),
            visual_options_panel: VisualOptionsPanel::new(),
            ant_status_bar: AntStatusBar::new(),
            dialog_popup: None,
            selected_ant: None,
            camera_locked_on_ant: None,
            top_panel_visible: true,
        }
    }

    pub fn select_ant(&mut self, ant_ref_option: Option<AntRef>) {
        self.selected_ant = ant_ref_option;
        if let Some(selected_ref) = &self.selected_ant {
            self.camera_locked_on_ant = Some(selected_ref.clone());
            if !self.debug_panel.is_enabled() {
                self.debug_panel.toggle();
            }
        } else {
            self.camera_locked_on_ant = None;
        }
    }

    pub fn deselect_ant(&mut self) {
        self.selected_ant = None;
        self.camera_locked_on_ant = None;
    }

    pub fn toggle_camera_lock(&mut self) {
        if self.camera_locked_on_ant.is_some() {
            self.camera_locked_on_ant = None;
        } else if let Some(selected_ref) = &self.selected_ant {
            self.camera_locked_on_ant = Some(selected_ref.clone());
        }
    }

    pub fn unlock_camera(&mut self) {
        self.camera_locked_on_ant = None;
    }

    pub fn get_selected_ant_ref(&self) -> Option<&AntRef> {
        self.selected_ant.as_ref()
    }

    pub fn is_camera_locked(&self) -> bool {
        self.camera_locked_on_ant.is_some()
            && self.selected_ant.is_some()
            && self.camera_locked_on_ant == self.selected_ant
    }

    pub fn get_camera_locked_ant_ref(&self) -> Option<&AntRef> {
        self.camera_locked_on_ant.as_ref()
    }

    pub fn handle_dead_ant(&mut self, dead_ant_key: crate::simulation::ant::AntKey) {
        if self
            .selected_ant
            .as_ref()
            .map_or(false, |r| r.key == dead_ant_key)
        {
            self.selected_ant = None;
        }
        if self
            .camera_locked_on_ant
            .as_ref()
            .map_or(false, |r| r.key == dead_ant_key)
        {
            self.camera_locked_on_ant = None;
        }
    }

    pub fn update(
        &mut self,
        editor: &mut EditorManager,
        simulation: &Simulation,
        camera: &mut GameCamera,
    ) -> (Option<AppAction>, bool) {
        let window_w = screen_width();
        let window_h = screen_height();
        // Only update camera on resize
        if (window_w, window_h) != self.last_screen_size {
            self.last_screen_size = (window_w, window_h);
            camera.handle_resize();
        }

        self.debug_panel.update();
        // Animate top panel every frame
        self.top_panel.update_animation(self.top_panel_visible);

        let mut input_consumed = false;
        let mut app_action = None;
        let mut ui_event_from_closure = None;

        let world_pos = camera.get_mouse_world_pos();

        let selected_ant_data_for_debug_panel = self
            .selected_ant
            .as_ref()
            .and_then(|ant_ref| simulation.get_ant(ant_ref));

        let is_camera_locked_for_debug_panel = self.is_camera_locked();

        new_egui_macroquad::ui(|egui_ctx| {
            set_theme(egui_ctx, catppuccin_egui::MOCHA);
            // Auto-zoom only on window resize or DPI change
            let win_px = egui_ctx.screen_rect().size() * egui_ctx.pixels_per_point();
            if (win_px.x - self.last_win_px.x).abs() > 1.0
                || (win_px.y - self.last_win_px.y).abs() > 1.0
            {
                self.last_win_px = win_px;
                let target = auto_zoom(egui_ctx, egui::vec2(1920.0, 1080.0));
                egui_ctx.set_zoom_factor(target);
            }

            if let Some(dialog) = &mut self.dialog_popup {
                let dialog_still_open = dialog.draw(egui_ctx);
                if !dialog_still_open {
                    if let Some(result) = dialog.result.take() {
                        match result {
                            DialogPopupResult::InputConfirmed(name) => {
                                if let DialogPopupMode::Input { label, .. } = &dialog.mode {
                                    if label.contains("save") {
                                        app_action = Some(AppAction::RequestSaveMap(name));
                                    } else if label.contains("load") {
                                        app_action = Some(AppAction::RequestLoadMap(name));
                                    }
                                }
                            }
                            DialogPopupResult::Confirmed => {
                                app_action = Some(AppAction::RequestReset);
                            }
                            DialogPopupResult::Cancelled => {}
                            DialogPopupResult::InfoOk => {}
                            DialogPopupResult::NewMapConfirmed { width, height } => {
                                app_action = Some(AppAction::RequestNewMap { width, height });
                            }
                        }
                    }
                    self.dialog_popup = None;
                }
                input_consumed = true;
            } else {
                let (new_ui_event, new_app_action, consumed_by_components) = self
                    .draw_ui_components(
                        egui_ctx,
                        editor,
                        simulation,
                        camera,
                        selected_ant_data_for_debug_panel,
                        is_camera_locked_for_debug_panel,
                    );
                if new_ui_event.is_some() {
                    ui_event_from_closure = new_ui_event;
                }
                if new_app_action.is_some() {
                    app_action = new_app_action;
                }
                input_consumed = consumed_by_components || egui_ctx.is_pointer_over_area();
                self.update_drag_state(egui_ctx);

                if !self.drag_started_on_ui && !egui_ctx.is_pointer_over_area() {
                    self.draw_pheromone_level_tooltip(egui_ctx, simulation, world_pos);
                    self.draw_colony_nest_hover_overlay(egui_ctx, simulation, camera);
                }
            }
        });

        if let Some(event) = ui_event_from_closure {
            match event {
                UIEvent::ToolSelected(tool) => editor.set_tool(tool),
                UIEvent::ToolSizeChanged(size) => editor.set_tool_size(size),
                UIEvent::ColorSelected(index) => editor.color_palette.set_selected_index(index),
                UIEvent::ToggleDebugPanel => self.toggle_debug_panel(),
                UIEvent::ToggleVisualOptionsPanel => self.toggle_visual_options_panel(),
                UIEvent::ShowNewMapDialog => self.show_dialog(DialogPopup::new_new_map(
                    DEFAULT_MAP_WIDTH,
                    DEFAULT_MAP_HEIGHT,
                )),
                UIEvent::ShowResetConfirmDialog => self.show_dialog(DialogPopup::new_confirm(
                    "Are you sure you want to reset the simulation?",
                )),
                UIEvent::ToggleTopPanel => {
                    self.top_panel_visible = !self.top_panel_visible;
                }
            }
        }

        (app_action, input_consumed || self.drag_started_on_ui)
    }

    pub fn show_dialog(&mut self, dialog: DialogPopup) {
        self.dialog_popup = Some(dialog);
    }

    fn update_drag_state(&mut self, egui_ctx: &egui::Context) {
        if is_mouse_button_down(MouseButton::Left) && egui_ctx.is_pointer_over_area() {
            self.drag_started_on_ui = true;
        } else if !is_mouse_button_down(MouseButton::Left) {
            self.drag_started_on_ui = false;
        }
    }

    fn draw_ui_components(
        &mut self,
        egui_ctx: &egui::Context,
        editor: &mut EditorManager,
        simulation: &Simulation,
        camera: &GameCamera,
        selected_ant_data: Option<&Ant>,
        is_camera_locked: bool,
    ) -> (Option<UIEvent>, Option<AppAction>, bool) {
        let mut ui_event = None;
        let mut app_action = None;
        let mut input_consumed = false;
        let mut top_panel_bottom_y = 0.0;

        if self.top_panel_visible || self.top_panel.animation_progress > 0.01 {
            let (panel_ui_event, panel_app_action, tool_consumed, panel_bottom_y) =
                self.top_panel.draw(
                    egui_ctx,
                    editor,
                    simulation,
                    &self.debug_panel,
                    &self.visual_options_panel,
                );

            if panel_ui_event.is_some() {
                ui_event = panel_ui_event;
            }
            if panel_app_action.is_some() {
                app_action = panel_app_action;
            }
            input_consumed |= tool_consumed;
            top_panel_bottom_y = panel_bottom_y;
        }

        let bar_offset = 6.0;
        let y_offset = if self.top_panel_visible {
            top_panel_bottom_y + bar_offset
        } else {
            0.0
        };
        if let Some(toggle_event) = self.top_panel.draw_toggle_bar_always(
            egui_ctx,
            self.top_panel.animation_progress > 0.01,
            y_offset,
        ) {
            if ui_event.is_none() {
                ui_event = Some(toggle_event);
            }
        }

        let debug_panel_action = self.debug_panel.draw(
            egui_ctx,
            simulation,
            camera,
            selected_ant_data,
            is_camera_locked,
        );
        if debug_panel_action.is_some() {
            app_action = debug_panel_action;
        }

        let colonies: Vec<(u32, egui::Color32)> = simulation
            .colonies
            .values()
            .map(|colony| {
                (
                    colony.colony_id,
                    egui::Color32::from_rgba_premultiplied(
                        (colony.color.r * 255.0) as u8,
                        (colony.color.g * 255.0) as u8,
                        (colony.color.b * 255.0) as u8,
                        255,
                    ),
                )
            })
            .collect();
        self.visual_options_panel.draw(egui_ctx, &colonies);

        // Draw the ant status bar at the bottom
        self.ant_status_bar.draw(egui_ctx, simulation);

        (ui_event, app_action, input_consumed)
    }

    pub fn render(&self) {
        new_egui_macroquad::draw();
    }

    pub fn toggle_top_panel(&mut self) {
        self.top_panel_visible = !self.top_panel_visible;
    }

    pub fn toggle_debug_panel(&mut self) {
        self.debug_panel.toggle();
    }

    pub fn toggle_visual_options_panel(&mut self) {
        self.visual_options_panel.toggle();
    }

    pub fn pheromone_display_mode(&self) -> PheromoneDisplayMode {
        self.visual_options_panel.pheromone_mode
    }

    pub fn show_ants(&self) -> bool {
        self.visual_options_panel.show_ants
    }

    pub fn time_multiplier(&self) -> Option<f32> {
        self.debug_panel.time_multiplier.or(Some(1.0))
    }

    pub fn unlimited(&self) -> bool {
        self.debug_panel.unlimited
    }

    fn draw_pheromone_level_tooltip(
        &self,
        egui_ctx: &egui::Context,
        simulation: &Simulation,
        world_pos: Vec2,
    ) {
        let (tile_x, tile_y) = (world_pos.x.floor() as usize, world_pos.y.floor() as usize);
        if !(tile_x < simulation.map.width as usize && tile_y < simulation.map.height as usize) {
            return;
        }
        let pheromone_mode = self.pheromone_display_mode();
        let level_to_display = match pheromone_mode {
            PheromoneDisplayMode::Channel { colony_id, channel } => {
                if let Some(colony) = simulation.colonies.get(&colony_id) {
                    let level = colony.get_pheromone_channel_at(
                        tile_x,
                        tile_y,
                        channel.saturating_sub(1) as usize,
                    );
                    if level > 0.0 { Some(level) } else { None }
                } else {
                    None
                }
            }
            _ => None,
        };
        if let Some(level) = level_to_display {
            let tooltip_text = format!("{:.2}", level);
            let screen_pos = egui_ctx
                .input(|i| i.pointer.hover_pos())
                .unwrap_or_default();
            let target_pos = screen_pos + egui::vec2(0.0, -12.0);
            let layer_id =
                egui::LayerId::new(egui::Order::Tooltip, "pheromone_tooltip_text".into());
            let painter = egui_ctx.layer_painter(layer_id);
            let text_color = egui_ctx.style().visuals.text_color();
            let font_id = egui::FontId::proportional(24.0);
            let text_galley =
                egui_ctx.fonts(|f| f.layout_no_wrap(tooltip_text, font_id, text_color));
            let text_pos = egui::pos2(
                target_pos.x - text_galley.size().x / 2.0,
                target_pos.y - text_galley.size().y,
            );
            painter.galley(text_pos, text_galley, text_color);
        }
    }

    fn draw_colony_nest_hover_overlay(
        &self,
        egui_ctx: &egui::Context,
        simulation: &Simulation,
        camera: &GameCamera,
    ) {
        let mouse_world = camera.get_mouse_world_pos();
        let mut hovered_colony: Option<&str> = None;
        for colony in simulation.colonies.values() {
            let dist = (colony.pos - mouse_world).length();
            if dist <= crate::simulation::COLONY_NEST_SIZE / 2.0 {
                hovered_colony = Some(&colony.player_config.name);
                break;
            }
        }
        if let Some(name) = hovered_colony {
            let screen_pos = egui_ctx
                .input(|i| i.pointer.hover_pos())
                .unwrap_or_default();
            let target_pos = screen_pos + egui::vec2(0.0, -12.0);
            let layer_id =
                egui::LayerId::new(egui::Order::Tooltip, "colony_nest_hover_text".into());
            let painter = egui_ctx.layer_painter(layer_id);
            let text_color = egui_ctx.style().visuals.text_color();
            let font_id = egui::FontId::proportional(24.0);
            let text_galley =
                egui_ctx.fonts(|f| f.layout_no_wrap(name.to_string(), font_id, text_color));
            let text_pos = egui::pos2(
                target_pos.x - text_galley.size().x / 2.0,
                target_pos.y - text_galley.size().y,
            );
            painter.galley(text_pos, text_galley, text_color);
        }
    }
}
