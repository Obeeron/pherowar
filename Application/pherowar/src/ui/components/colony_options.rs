use new_egui_macroquad::egui;
use new_egui_macroquad::egui::Color32;

use crate::editor::EditorManager;
use crate::editor::ToolType;
use crate::editor::color_palette::{ColorPalette, PREDEFINED_COLONY_COLORS};
use crate::simulation::Simulation;
use crate::ui::BASE_SPACING;
use crate::ui::events::UIEvent;

/// UI component for selecting player and colony color for placement.
pub struct ColonyOptions {}

impl ColonyOptions {
    pub fn new() -> Self {
        Self {}
    }

    /// Draws the colony options UI (player selector, color palette).
    pub fn draw(
        &self,
        ui: &mut egui::Ui,
        editor_manager: &mut EditorManager,
        simulation: &Simulation,
    ) -> Option<UIEvent> {
        let mut ui_event: Option<UIEvent> = None;

        ui.horizontal(|ui| {
            ui.label("Player:");
            let selected_player_label = if editor_manager
                .current_player_index()
                .map_or(false, |idx| idx == 0)
            {
                "Placeholder".to_string()
            } else if let Some(player_idx_1_based) = editor_manager.current_player_index() {
                if player_idx_1_based > 0 {
                    let actual_idx = player_idx_1_based - 1;
                    if actual_idx < simulation.player_configs.len() {
                        simulation.player_configs[actual_idx].name.clone()
                    } else {
                        "Invalid Player".to_string() // Should not happen with valid indices
                    }
                } else {
                    "Placeholder".to_string() // Index 0 is placeholder
                }
            } else {
                "None".to_string() // No player/placeholder selected
            };

            egui::ComboBox::from_id_source("player_select")
                .selected_text(selected_player_label)
                .show_ui(ui, |ui| {
                    // Placeholder option (index 0)
                    let is_placeholder_selected = editor_manager
                        .current_player_index()
                        .map_or(false, |idx| idx == 0);
                    if ui
                        .selectable_label(is_placeholder_selected, "Placeholder")
                        .clicked()
                    {
                        editor_manager.set_player(Some(0));
                        ui_event = Some(UIEvent::ToolSelected(Some(ToolType::Colony)));
                    }

                    // Player options (1-based index for UI)
                    for (i, config) in simulation.player_configs.iter().enumerate() {
                        let player_option_idx = i + 1;
                        let is_selected = editor_manager
                            .current_player_index()
                            .map_or(false, |idx| idx == player_option_idx);
                        if ui.selectable_label(is_selected, &config.name).clicked() {
                            editor_manager.set_player(Some(player_option_idx));
                            ui_event = Some(UIEvent::ToolSelected(Some(ToolType::Colony)));
                        }
                    }
                });
        });

        let all_colors_currently_used = ColorPalette::are_all_colors_used(simulation);

        // Auto-select an available color if the current palette selection is already in use by a colony.
        if !PREDEFINED_COLONY_COLORS.is_empty() {
            let current_selected_idx = editor_manager.color_palette.get_selected_index();
            let current_selected_color_value = PREDEFINED_COLONY_COLORS[current_selected_idx];

            if ColorPalette::is_color_used(current_selected_color_value, simulation) {
                if let Some(first_available_index) = PREDEFINED_COLONY_COLORS
                    .iter()
                    .position(|&color| !ColorPalette::is_color_used(color, simulation))
                {
                    editor_manager
                        .color_palette
                        .set_selected_index(first_available_index);
                    if ui_event.is_none() {
                        ui_event = Some(UIEvent::ColorSelected(first_available_index));
                    }
                }
            }
        }

        ui.add_space(BASE_SPACING);
        ui.label("Colony Color:");

        // Color Palette Display
        ui.horizontal_wrapped(|ui| {
            for (index, &color_val_macroquad) in PREDEFINED_COLONY_COLORS.iter().enumerate() {
                let is_selected = editor_manager.color_palette.get_selected_index() == index;

                let color_val_egui = Color32::from_rgb(
                    (color_val_macroquad.r * 255.0) as u8,
                    (color_val_macroquad.g * 255.0) as u8,
                    (color_val_macroquad.b * 255.0) as u8,
                );

                let is_used = ColorPalette::is_color_used(color_val_macroquad, simulation);

                let (stroke_color, stroke_width) = if all_colors_currently_used {
                    (Color32::GRAY, 1.0)
                } else {
                    if is_selected {
                        (Color32::WHITE, 2.0)
                    } else {
                        (Color32::GRAY, 1.0)
                    }
                };

                let color_button_widget = egui::Button::new("")
                    .fill(color_val_egui)
                    .stroke(egui::Stroke::new(stroke_width, stroke_color));

                let enabled = !is_used; // Button is enabled if color is not used.
                let desired_button_size =
                    egui::vec2(ui.spacing().interact_size.y, ui.spacing().interact_size.y);

                // Add button, disabled if color is used.
                let response = ui
                    .add_enabled_ui(enabled, |ui| {
                        ui.add_sized(desired_button_size, color_button_widget)
                    })
                    .inner;

                if response.clicked() {
                    // `clicked()` respects the enabled state.
                    ui_event = Some(UIEvent::ColorSelected(index));
                }

                // Add a visual dark circle cue if the color is used.
                if is_used {
                    let painter = ui.painter();
                    let rect = response.rect;
                    painter.circle_filled(rect.center(), rect.width() * 0.25, Color32::DARK_GRAY);
                }
            }
        });

        ui_event
    }
}
