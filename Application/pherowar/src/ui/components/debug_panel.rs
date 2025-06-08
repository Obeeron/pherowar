use crate::engine::GameCamera;
use crate::simulation::ant::Ant;
use crate::simulation::{MAX_TIME_MULTIPLIER, MIN_TIME_MULTIPLIER, Simulation};
use crate::ui::events::AppAction;
use crate::ui::{BASE_PADDING, BASE_SPACING};
use egui::RichText;
use macroquad::prelude::*;
use new_egui_macroquad::egui;
use new_egui_macroquad::egui::Color32;
use shared::MEMORY_SIZE;

/// Debug panel component that displays debug information
pub struct DebugPanel {
    displayed_fps: i32,
    fps_timer: f32,
    show_debug: bool,
    pub time_multiplier: Option<f32>, // None = 1.0x, Some(x) = custom
    pub unlimited: bool,
}

impl DebugPanel {
    pub fn new() -> Self {
        Self {
            displayed_fps: get_fps(),
            fps_timer: 0.0,
            show_debug: false,
            time_multiplier: Some(1.0),
            unlimited: false,
        }
    }

    /// Update the FPS counter
    pub fn update(&mut self) {
        self.fps_timer += get_frame_time();
        if self.fps_timer >= 0.5 {
            self.displayed_fps = get_fps();
            self.fps_timer = 0.0;
        }
    }

    /// Check if debug panel is enabled
    pub fn is_enabled(&self) -> bool {
        return self.show_debug;
    }

    /// Toggle debug panel visibility
    pub fn toggle(&mut self) -> bool {
        self.show_debug = !self.show_debug;
        return self.show_debug;
    }

    /// Draw the debug panel
    pub fn draw(
        &mut self,
        egui_ctx: &egui::Context,
        simulation: &Simulation,
        camera: &GameCamera,
        selected_ant: Option<&Ant>,
        is_camera_locked: bool,
    ) -> Option<AppAction> {
        if !self.show_debug {
            return None;
        }

        let mut app_action = None;

        egui::Window::new("Debug Info")
            .resizable(true)
            .collapsible(true)
            .default_pos(egui::pos2(screen_width() - 320.0, 32.0 + 6.0 * 2.0))
            .default_size(egui::vec2(300.0, screen_height() * 0.7))
            .show(egui_ctx, |ui| {
                ui.heading("Performance");
                ui.group(|ui| {
                    let fps_color = if self.displayed_fps > 55 {
                        Color32::from_rgb(0, 180, 0)
                    } else if self.displayed_fps > 30 {
                        Color32::from_rgb(220, 180, 70)
                    } else {
                        Color32::from_rgb(220, 100, 100)
                    };

                    egui::Grid::new("perf_grid")
                        .num_columns(2)
                        .spacing([BASE_SPACING * 2.0, BASE_SPACING])
                        .show(ui, |ui| {
                            ui.label("FPS:");
                            ui.colored_label(fps_color, format!("{}", self.displayed_fps));
                            ui.end_row();
                        });

                    ui.horizontal(|ui| {
                        ui.label("Unlimited simulation speed:");
                        if ui.checkbox(&mut self.unlimited, "").changed() {
                            if self.unlimited {
                                self.time_multiplier = None;
                            } else {
                                self.time_multiplier = Some(1.0);
                            }
                        }
                    });

                    let mut multiplier_val = self.time_multiplier.unwrap_or(1.0);
                    let slider = egui::Slider::new(
                        &mut multiplier_val,
                        MIN_TIME_MULTIPLIER..=MAX_TIME_MULTIPLIER,
                    )
                    .clamp_to_range(true)
                    .logarithmic(true)
                    .custom_formatter(|n, _decimals| format!("{:.2}x", n));
                    if ui.add_enabled(!self.unlimited, slider).changed() && !self.unlimited {
                        self.time_multiplier = Some(multiplier_val.max(MIN_TIME_MULTIPLIER));
                    }
                });

                ui.add_space(BASE_PADDING);
                ui.heading("Simulation");
                ui.group(|ui| {
                    egui::Grid::new("sim_stats")
                        .num_columns(2)
                        .spacing([BASE_SPACING * 2.0, BASE_SPACING])
                        .show(ui, |ui| {
                            ui.label("Tick:");
                            ui.label(simulation.tick.to_string());
                            ui.end_row();

                            let mouse_pos = camera.get_mouse_world_pos();
                            ui.label("Mouse:");
                            ui.label(format!("({:.1}, {:.1})", mouse_pos.x, mouse_pos.y));
                            ui.end_row();

                            // Add map size info
                            ui.label("Map Size:");
                            ui.label(format!(
                                "{} x {}",
                                simulation.map.width, simulation.map.height
                            ));
                            ui.end_row();

                            ui.label("Colonies:");
                            ui.label(simulation.colonies.len().to_string());
                            ui.end_row();

                            let total_ants = simulation
                                .colonies
                                .values()
                                .map(|c| c.ants.len())
                                .sum::<usize>();
                            ui.label("Total Ants:");
                            ui.label(total_ants.to_string());
                            ui.end_row();

                            if !simulation.colonies.is_empty() {
                                ui.separator();
                                ui.end_row();

                                for (id, colony) in &simulation.colonies {
                                    let name = &colony.player_config.name;

                                    let colony_color = egui::Color32::from_rgba_unmultiplied(
                                        (colony.color.r * 255.0) as u8,
                                        (colony.color.g * 255.0) as u8,
                                        (colony.color.b * 255.0) as u8,
                                        255,
                                    );

                                    ui.horizontal(|ui| {
                                        ui.colored_label(colony_color, format!("\"{}\"", name));
                                    });
                                    ui.label(format!("  Ants: {}", colony.ants.len()));
                                    ui.end_row();

                                    ui.label("");
                                    ui.label(format!("  Food: {}", colony.food_collected));
                                    ui.end_row();
                                }
                            }
                        });
                });

                if let Some(ant) = selected_ant {
                    ui.add_space(BASE_PADDING);
                    ui.heading("Selected Ant:");
                    ui.group(|ui| {
                        egui::Grid::new("ant_info_grid")
                            .num_columns(2)
                            .spacing([BASE_SPACING * 2.0, BASE_SPACING])
                            .show(ui, |ui| {
                                ui.label("Colony ID:");
                                if let Some(colony) =
                                    simulation.colonies.get(&ant.ant_ref.colony_id)
                                {
                                    let colony_color = egui::Color32::from_rgba_unmultiplied(
                                        (colony.color.r * 255.0) as u8,
                                        (colony.color.g * 255.0) as u8,
                                        (colony.color.b * 255.0) as u8,
                                        255,
                                    );
                                    ui.horizontal(|ui| {
                                        ui.colored_label(
                                            colony_color,
                                            format!("{}", ant.ant_ref.colony_id),
                                        );
                                        ui.label(format!(" ({})", colony.player_config.name));
                                    });
                                } else {
                                    ui.label(format!("{}", ant.ant_ref.colony_id));
                                }
                                ui.end_row();

                                ui.label("Position:");
                                ui.label(format!("({:.1}, {:.1})", ant.pos.x, ant.pos.y));
                                ui.end_row();

                                ui.label("Rotation:");
                                ui.label(format!(
                                    "{:.2} rad / {:.2} degrees",
                                    ant.rotation,
                                    ant.rotation.to_degrees()
                                ));
                                ui.end_row();

                                ui.label("is_carrying_food:");
                                ui.label(ant.carrying_food.to_string());
                                ui.end_row();

                                ui.label("is_on_food:");
                                ui.label(ant.is_on_food.to_string());
                                ui.end_row();

                                ui.label("is_on_colony:");
                                ui.label(ant.is_on_colony.to_string());
                                ui.end_row();

                                ui.label("longevity:");
                                ui.label(ant.longevity.to_string());
                                ui.end_row();

                                ui.label("Fighting:");
                                if !ant.fight_opponents.is_empty() {
                                    ui.label(format!(
                                        "Yes ({} opponents)",
                                        ant.fight_opponents.len()
                                    ));
                                } else {
                                    ui.label("No");
                                }
                                ui.end_row();
                            });

                        ui.add_space(BASE_SPACING);
                        ui.strong("Memory View (Hex):");
                        egui::Frame::dark_canvas(ui.style()).show(ui, |ui| {
                            egui::ScrollArea::vertical()
                                .max_height(100.0)
                                .show(ui, |ui| {
                                    const BYTES_PER_LINE: usize = 8;
                                    const GROUP_SIZE: usize = 4;
                                    for line_start in (0..MEMORY_SIZE).step_by(BYTES_PER_LINE) {
                                        ui.horizontal_wrapped(|ui| {
                                            ui.label(
                                                RichText::new(format!("{:02X}:", line_start))
                                                    .monospace()
                                                    .color(Color32::GRAY),
                                            );
                                            ui.add_space(ui.spacing().item_spacing.x);

                                            for i in 0..BYTES_PER_LINE {
                                                if (line_start + i) < MEMORY_SIZE {
                                                    if i > 0 && i % GROUP_SIZE == 0 {
                                                        ui.add_space(
                                                            ui.spacing().item_spacing.x * 1.5,
                                                        );
                                                    }
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "{:02X}",
                                                            ant.memory[line_start + i]
                                                        ))
                                                        .monospace(),
                                                    );
                                                } else {
                                                    if i > 0 && i % GROUP_SIZE == 0 {
                                                        ui.add_space(
                                                            ui.spacing().item_spacing.x * 1.5,
                                                        );
                                                    }
                                                    ui.label(RichText::new("  ").monospace());
                                                }
                                            }
                                        });
                                    }
                                });
                        });

                        ui.add_space(BASE_SPACING);
                        let button_text = if is_camera_locked {
                            "Unlock Camera from Ant"
                        } else {
                            "Lock Camera on Ant"
                        };
                        if ui.button(button_text).clicked() {
                            app_action = Some(AppAction::ToggleCameraLockOnSelectedAnt);
                        }
                    });
                }
            });
        app_action
    }
}
