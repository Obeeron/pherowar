use crate::ui::BASE_PADDING;
use new_egui_macroquad::egui;

/// Visual options for pheromone display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PheromoneDisplayMode {
    None,
    Colony { colony_id: u32 },
    Channel { colony_id: u32, channel: u8 },
}

/// Visual options panel component
pub struct VisualOptionsPanel {
    show_visual_options: bool,
    pub pheromone_mode: PheromoneDisplayMode,
    pub selected_colony_id: Option<u32>, // For both modes
    pub selected_channel: u8,            // For Channel mode
    pub show_ants: bool,
}

impl VisualOptionsPanel {
    pub fn new() -> Self {
        Self {
            show_visual_options: false,
            pheromone_mode: PheromoneDisplayMode::None,
            selected_colony_id: None,
            selected_channel: 1,
            show_ants: true,
        }
    }

    /// Check if visual options panel is enabled
    pub fn is_enabled(&self) -> bool {
        self.show_visual_options
    }

    /// Toggle visual options panel visibility
    pub fn toggle(&mut self) -> bool {
        self.show_visual_options = !self.show_visual_options;
        self.show_visual_options
    }

    /// Draw the visual options panel
    pub fn draw(&mut self, egui_ctx: &egui::Context, colonies: &[(u32, egui::Color32)]) {
        if !self.show_visual_options {
            return;
        }
        egui::Window::new("Visual Options")
            .resizable(false)
            .collapsible(true)
            .default_pos(egui::pos2(32.0, 32.0))
            .default_size(egui::vec2(260.0, 240.0))
            .show(egui_ctx, |ui| {
                ui.heading("Ants");
                ui.checkbox(&mut self.show_ants, "Draw Ants");
                ui.add_space(BASE_PADDING);

                ui.heading("Pheromones");
                ui.horizontal(|ui| {
                    let hide_selected = matches!(self.pheromone_mode, PheromoneDisplayMode::None);
                    let colony_selected =
                        matches!(self.pheromone_mode, PheromoneDisplayMode::Colony { .. });
                    let channel_selected =
                        matches!(self.pheromone_mode, PheromoneDisplayMode::Channel { .. });

                    if ui.selectable_label(hide_selected, "Hide").clicked() {
                        self.pheromone_mode = PheromoneDisplayMode::None;
                    }
                    if ui.selectable_label(colony_selected, "Colony").clicked() {
                        if !colony_selected {
                            if let Some((colony_id, _)) = colonies.first() {
                                self.selected_colony_id = Some(*colony_id);
                                self.pheromone_mode = PheromoneDisplayMode::Colony {
                                    colony_id: *colony_id,
                                };
                            }
                        }
                    }
                    if ui.selectable_label(channel_selected, "Channel").clicked() {
                        if !channel_selected {
                            if let Some((colony_id, _)) = colonies.first() {
                                self.selected_colony_id = Some(*colony_id);
                                self.pheromone_mode = PheromoneDisplayMode::Channel {
                                    colony_id: *colony_id,
                                    channel: self.selected_channel,
                                };
                            }
                        }
                    }
                });
                // Always keep one selected
                if !matches!(
                    self.pheromone_mode,
                    PheromoneDisplayMode::None
                        | PheromoneDisplayMode::Colony { .. }
                        | PheromoneDisplayMode::Channel { .. }
                ) {
                    self.pheromone_mode = PheromoneDisplayMode::None;
                }
                match self.pheromone_mode {
                    PheromoneDisplayMode::Colony { .. } | PheromoneDisplayMode::Channel { .. } => {
                        ui.label("Select Colony:");
                        egui::Grid::new("colony_color_grid_visual_opts")
                            .spacing([8.0, 8.0])
                            .min_col_width(24.0)
                            .show(ui, |ui| {
                                let columns = 6;
                                let mut col_count = 0;
                                for (colony_id, color32) in colonies.iter() {
                                    let is_selected = self.selected_colony_id == Some(*colony_id);
                                    let button = ui.add_sized(
                                        egui::vec2(24.0, 24.0),
                                        egui::Button::new("").fill(*color32).stroke(
                                            if is_selected {
                                                egui::Stroke::new(2.0, egui::Color32::WHITE)
                                            } else {
                                                egui::Stroke::NONE
                                            },
                                        ),
                                    );
                                    if button.clicked() {
                                        self.selected_colony_id = Some(*colony_id);
                                        match self.pheromone_mode {
                                            PheromoneDisplayMode::Colony { .. } => {
                                                self.pheromone_mode =
                                                    PheromoneDisplayMode::Colony {
                                                        colony_id: *colony_id,
                                                    };
                                            }
                                            PheromoneDisplayMode::Channel { channel, .. } => {
                                                self.pheromone_mode =
                                                    PheromoneDisplayMode::Channel {
                                                        colony_id: *colony_id,
                                                        channel,
                                                    };
                                            }
                                            _ => {}
                                        }
                                    }
                                    col_count += 1;
                                    if col_count % columns == 0 {
                                        ui.end_row();
                                    }
                                }
                            });
                    }
                    _ => {}
                }
                if let PheromoneDisplayMode::Channel {
                    colony_id: _colony_id,
                    ..
                } = &mut self.pheromone_mode
                {
                    ui.label("Select Channel:");
                    for ch_val in 1..=8 {
                        let channel_u8 = ch_val as u8;
                        if ui
                            .radio_value(
                                &mut self.selected_channel,
                                channel_u8,
                                format!("Channel {}", ch_val),
                            )
                            .clicked()
                        {
                            // Update pheromone_mode when a radio button is clicked
                            self.pheromone_mode = PheromoneDisplayMode::Channel {
                                colony_id: self.selected_colony_id.unwrap_or_default(),
                                channel: self.selected_channel,
                            };
                        }
                    }
                }
            });
    }
}
