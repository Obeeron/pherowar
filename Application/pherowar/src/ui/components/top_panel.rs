use catppuccin_egui;
use epaint::Margin;
use macroquad::prelude::*;
use new_egui_macroquad::egui::{self, epaint};

use crate::editor::symmetry_mode::SymmetryMode;
use crate::editor::{EditorManager, ToolType};
use crate::simulation::Simulation;
use crate::ui::components::{ColonyOptions, ToolSizeSlider};
use crate::ui::events::{AppAction, UIEvent};
use crate::ui::{
    BASE_BUTTON_HEIGHT, BASE_BUTTON_WIDTH, BASE_ICON_SIZE, BASE_PADDING, BASE_SPACING,
};

/// Component for the main tool panel at the top of the screen
pub struct TopPanel {
    tool_size_slider: ToolSizeSlider,
    colony_options: ColonyOptions,
    pub animation_progress: f32, // 0.0 = hidden, 1.0 = shown
    pub animation_target: f32,   // 0.0 = hidden, 1.0 = shown
}

impl TopPanel {
    pub fn new() -> Self {
        Self {
            tool_size_slider: ToolSizeSlider::new(),
            colony_options: ColonyOptions::new(),
            animation_progress: 1.0,
            animation_target: 1.0,
        }
    }

    /// Call this every frame to update the animation progress
    pub fn update_animation(&mut self, visible: bool) {
        self.animation_target = if visible { 1.0 } else { 0.0 };
        // Fast-out: exponential ease, slower
        let speed = 0.15; // Lower = slower
        if (self.animation_progress - self.animation_target).abs() > 0.001 {
            self.animation_progress += (self.animation_target - self.animation_progress) * speed;
            if (self.animation_progress - self.animation_target).abs() < 0.001 {
                self.animation_progress = self.animation_target;
            }
        }
    }

    fn icon_button(&self, ui: &mut egui::Ui, label: &str, active: bool) -> egui::Response {
        let mut button = egui::Button::new(egui::RichText::new(label).size(20.0));
        if active {
            button = button.fill(catppuccin_egui::MOCHA.overlay0);
        }
        ui.add_sized([BASE_ICON_SIZE, BASE_ICON_SIZE], button)
    }

    fn draw_help_tooltip(&self, egui_ctx: &egui::Context) {
        if let Some(mouse_pos) = egui_ctx.input(|i| i.pointer.hover_pos()) {
            egui::Window::new("")
                .title_bar(false)
                .collapsible(false)
                .resizable(false)
                .fixed_pos(mouse_pos + egui::vec2(10.0, 10.0))
                .show(egui_ctx, |ui| {
                    ui.heading("Keyboard Shortcuts");
                    ui.add_space(BASE_PADDING);
                    egui::Grid::new("keyboard_shortcuts")
                        .num_columns(2)
                        .spacing([BASE_SPACING * 2.0, BASE_SPACING])
                        .striped(true)
                        .show(ui, |ui| {
                            for (key, action) in self.keyboard_shortcuts() {
                                ui.monospace(key);
                                ui.label(action);
                                ui.end_row();
                            }
                        });
                    ui.add_space(BASE_PADDING * 2.0);
                    ui.heading("Mouse Controls");
                    ui.add_space(BASE_PADDING);
                    egui::Grid::new("mouse_controls")
                        .num_columns(2)
                        .spacing([BASE_SPACING * 2.0, BASE_SPACING])
                        .striped(true)
                        .show(ui, |ui| {
                            for (action, desc) in self.mouse_controls() {
                                ui.monospace(action);
                                ui.label(desc);
                                ui.end_row();
                            }
                        });
                });
        }
    }

    fn keyboard_shortcuts(&self) -> Vec<(&'static str, &'static str)> {
        vec![
            ("1", "Select Food tool"),
            ("2", "Select Wall tool"),
            ("3", "Select Colony tool"),
            ("Esc", "Deselect tool / Close dialog"),
            ("P or Space", "Pause/resume simulation"),
            ("R", "Reset simulation"),
            ("S", "Save map"),
            ("L", "Load map"),
            ("F", "Toggle tool panel"),
            ("D", "Toggle debug panel"),
            ("V", "Toggle visual options panel"),
        ]
    }

    fn mouse_controls(&self) -> Vec<(&'static str, &'static str)> {
        vec![
            ("Scroll", "Zoom in/out"),
            ("Left Click", "Drag view / Use tool"),
            ("Ctrl+Left Click", "Drag view (alternative)"),
            ("Right Click", "Remove with tool"),
            ("Alt+Left Click or Double Click", "Select/deselect ant"),
        ]
    }

    pub fn draw_toggle_bar_always(
        &self,
        egui_ctx: &egui::Context,
        top_panel_visible: bool,
        y_offset: f32,
    ) -> Option<UIEvent> {
        let base_width = 200.0;
        let hover_width = 220.0;
        let height = 10.0;
        let rounding = height * 0.5;
        let screen_width = egui_ctx.screen_rect().width();
        let mut width = base_width;
        let area_id = if top_panel_visible {
            "top_panel_toggle_bar"
        } else {
            "top_panel_toggle_bar_collapsed"
        };
        let mut event = None;
        let button_height = 18.0;
        let mut show_bar = !top_panel_visible;
        if top_panel_visible {
            let pointer_pos = egui_ctx.input(|i| i.pointer.hover_pos());
            let bar_rect = egui::Rect::from_min_size(
                egui::pos2((screen_width - hover_width) / 2.0, y_offset),
                egui::vec2(hover_width, button_height),
            );
            if let Some(pos) = pointer_pos {
                if bar_rect.contains(pos) {
                    show_bar = true;
                } else {
                    show_bar = false;
                }
            } else {
                show_bar = false;
            }
        }
        if !show_bar {
            return None;
        }
        egui::Area::new(egui::Id::new(area_id))
            .fixed_pos(egui::pos2((screen_width - hover_width) / 2.0, y_offset))
            .constrain(false)
            .order(egui::Order::Foreground)
            .show(egui_ctx, |ui| {
                let (rect, response) = ui.allocate_exact_size(
                    egui::vec2(hover_width, button_height),
                    egui::Sense::click(),
                );
                let is_hovered = response.hovered();
                width = if is_hovered { hover_width } else { base_width };
                let center_x = (hover_width - width) / 2.0;
                let color = if is_hovered {
                    egui::Color32::from_rgba_unmultiplied(180, 180, 220, 220)
                } else {
                    egui::Color32::from_rgba_unmultiplied(120, 120, 160, 180)
                };
                let pill_rect = egui::Rect::from_min_size(
                    rect.min + egui::vec2(center_x, (button_height - height) / 2.0),
                    egui::vec2(width, height),
                );
                ui.painter().rect_filled(pill_rect, rounding, color);
                if response.clicked() {
                    event = Some(UIEvent::ToggleTopPanel);
                }
                response.on_hover_text("Show/hide the tool bar (F)");
            });
        event
    }

    pub fn draw(
        &mut self,
        egui_ctx: &egui::Context,
        editor: &mut EditorManager,
        simulation: &Simulation,
        debug_panel: &crate::ui::components::DebugPanel,
        visual_options_panel: &crate::ui::components::VisualOptionsPanel,
    ) -> (Option<UIEvent>, Option<AppAction>, bool, f32) {
        let mut ui_event = None;
        let mut app_action = None;
        let mut input_consumed = false;
        let mut panel_bottom_y = 0.0;
        // Animate vertical offset
        let max_offset = 0.0;
        let min_offset = if editor.current_tool() != None {
            -120.0
        } else {
            -60.0
        }; // Hide panel further above screen
        let y_offset = min_offset + (max_offset - min_offset) * self.animation_progress;
        egui::Area::new(egui::Id::new("top_panel_area_anim"))
            .anchor(
                egui::Align2::CENTER_TOP,
                egui::Vec2::new(0.0, BASE_PADDING + y_offset),
            )
            .constrain(false)
            .order(egui::Order::Middle)
            .show(egui_ctx, |ui| {
                egui::Frame::none()
                    .fill(egui_ctx.style().visuals.panel_fill)
                    .inner_margin(Margin::same(BASE_PADDING))
                    .rounding(egui::Rounding::same(6.0))
                    .show(ui, |ui| {
                        ui.add_enabled_ui(self.animation_progress > 0.01, |ui| {
                            ui.add_visible_ui(self.animation_progress > 0.01, |ui| {
                                let current_tool = editor.current_tool();
                                let show_size = current_tool.map_or(false, |t| t.is_sizeable());
                                let show_colony =
                                    current_tool.map_or(false, |t| t == ToolType::Colony);
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = BASE_SPACING;
                                    for &tool in ToolType::all() {
                                        let button_size =
                                            egui::vec2(BASE_BUTTON_WIDTH, BASE_BUTTON_HEIGHT);
                                        let mut button = egui::Button::new(tool.label());
                                        if Some(tool) == current_tool {
                                            button = button.fill(catppuccin_egui::MOCHA.surface1);
                                        }
                                        let response = ui.add_sized(button_size, button);
                                        if response.clicked() {
                                            ui_event = Some(UIEvent::ToolSelected(Some(tool)));
                                            input_consumed = true;
                                        }
                                    }

                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.add_space(BASE_SPACING);
                                            let help_response = ui.add(
                                                egui::Label::new("❓").sense(egui::Sense::hover()),
                                            );
                                            if help_response.hovered() {
                                                self.draw_help_tooltip(egui_ctx);
                                            }
                                            ui.add_space(BASE_SPACING);
                                            let debug_btn = self
                                                .icon_button(ui, "🛠", debug_panel.is_enabled())
                                                .on_hover_text("Show/hide debug panel");
                                            if debug_btn.clicked() {
                                                ui_event = Some(UIEvent::ToggleDebugPanel);
                                                input_consumed = true;
                                            }
                                            let visual_btn = self
                                                .icon_button(
                                                    ui,
                                                    "👁",
                                                    visual_options_panel.is_enabled(),
                                                )
                                                .on_hover_text("Show/hide visual options");
                                            if visual_btn.clicked() {
                                                ui_event = Some(UIEvent::ToggleVisualOptionsPanel);
                                                input_consumed = true;
                                            }
                                            let new_map_btn = self
                                                .icon_button(ui, "⛶", false)
                                                .on_hover_text("Create new map");
                                            if new_map_btn.clicked() {
                                                ui_event = Some(UIEvent::ShowNewMapDialog);
                                                input_consumed = true;
                                            }
                                            let load_btn = self
                                                .icon_button(ui, "🗁", false)
                                                .on_hover_text("Load map");
                                            if load_btn.clicked() {
                                                app_action =
                                                    Some(AppAction::RequestLoadMap("".to_string()));
                                                input_consumed = true;
                                            }
                                            let save_btn = self
                                                .icon_button(ui, "💾", false)
                                                .on_hover_text("Save map");
                                            if save_btn.clicked() {
                                                app_action =
                                                    Some(AppAction::RequestSaveMap("".to_string()));
                                                input_consumed = true;
                                            }
                                            ui.add_space(2.0 * BASE_SPACING);
                                            let reset_btn = self
                                                .icon_button(ui, "🔄", false)
                                                .on_hover_text("Reset simulation");
                                            if reset_btn.clicked() {
                                                ui_event = Some(UIEvent::ShowResetConfirmDialog);
                                                input_consumed = true;
                                            }
                                            let pause_btn = self
                                                .icon_button(
                                                    ui,
                                                    if simulation.is_paused {
                                                        "▶"
                                                    } else {
                                                        "⏸"
                                                    },
                                                    false,
                                                )
                                                .on_hover_text("Pause/resume simulation");
                                            if pause_btn.clicked() {
                                                app_action = Some(AppAction::TogglePause);
                                                input_consumed = true;
                                            }
                                        },
                                    );
                                });
                                if current_tool.is_some() && (show_size || show_colony) {
                                    ui.add_space(BASE_SPACING);
                                    ui.separator();
                                    ui.add_space(BASE_SPACING);
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = BASE_SPACING * 1.5;
                                        if show_size {
                                            if let Some(tool_event) =
                                                self.tool_size_slider.draw(ui, editor)
                                            {
                                                if ui_event.is_none() {
                                                    ui_event = Some(tool_event);
                                                }
                                                input_consumed = true;
                                            }
                                        }
                                        if show_colony {
                                            // Updated call to colony_options.draw and handling of its result
                                            let colony_event =
                                                self.colony_options.draw(ui, editor, simulation);
                                            if colony_event.is_some() {
                                                if ui_event.is_none() {
                                                    ui_event = colony_event;
                                                }
                                                input_consumed = true; // Assume input is consumed if there's a colony event
                                            }
                                        }
                                        // Symmetry selector: compact, next to tool size/colony color
                                        ui.add_space(BASE_SPACING);
                                        ui.label(egui::RichText::new("Symmetry").strong());
                                        egui::ComboBox::from_id_source("symmetry_mode_selector")
                                            .width(80.0)
                                            .selected_text(editor.symmetry_mode.label())
                                            .show_ui(ui, |ui| {
                                                for &mode in SymmetryMode::ALL.iter() {
                                                    if ui
                                                        .selectable_label(
                                                            editor.symmetry_mode == mode,
                                                            mode.label(),
                                                        )
                                                        .clicked()
                                                    {
                                                        editor.symmetry_mode = mode;
                                                    }
                                                }
                                            });
                                    });
                                    ui.add_space(BASE_SPACING);
                                }
                                panel_bottom_y = ui.min_rect().bottom();
                            });
                        });
                    });
            });
        (ui_event, app_action, input_consumed, panel_bottom_y)
    }
}
