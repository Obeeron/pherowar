use crate::simulation::Simulation;
use new_egui_macroquad::egui;

pub struct AntStatusBar {}

impl AntStatusBar {
    pub fn new() -> Self {
        Self {}
    }

    pub fn draw(&mut self, ctx: &egui::Context, simulation: &Simulation) -> f32 {
        let total_ants = simulation.total_ant_count();

        if total_ants == 0 {
            return 0.0;
        }

        let mut colony_stats: Vec<(u32, usize, egui::Color32, String)> = simulation
            .colonies
            .values()
            .map(|colony| {
                let ant_count = colony.ants.len();
                let color = egui::Color32::from_rgba_premultiplied(
                    (colony.color.r * 255.0) as u8,
                    (colony.color.g * 255.0) as u8,
                    (colony.color.b * 255.0) as u8,
                    255,
                );
                let name = colony.player_config.name.clone();
                (colony.colony_id, ant_count, color, name)
            })
            .filter(|(_, ant_count, _, _)| *ant_count > 0)
            .collect();

        colony_stats.sort_by_key(|&(colony_id, _, _, _)| colony_id);

        let bar_height = 15.0;

        egui::TopBottomPanel::bottom("ant_status_bar")
            .exact_height(bar_height)
            .resizable(false)
            .show_separator_line(false)
            .frame(egui::Frame {
                stroke: egui::Stroke::NONE,
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    let available_width = ui.available_width();

                    for (_, ant_count, color, name) in colony_stats.iter() {
                        let percentage = *ant_count as f32 / total_ants as f32;
                        let segment_width = available_width * percentage;

                        let (rect, _) = ui.allocate_exact_size(
                            egui::Vec2::new(segment_width, bar_height),
                            egui::Sense::hover(),
                        );

                        ui.painter().rect_filled(rect, 0.0, *color);

                        let text = format!("{} ({} ants)", name, ant_count);
                        let text_color = egui::Color32::WHITE;
                        let font_id = egui::FontId::default();

                        let text_size = ui
                            .painter()
                            .layout_no_wrap(text.clone(), font_id.clone(), text_color)
                            .size();

                        if text_size.x <= rect.width() {
                            let text_pos = egui::Pos2::new(rect.center().x, rect.center().y);

                            ui.painter().text(
                                text_pos,
                                egui::Align2::CENTER_CENTER,
                                text,
                                font_id,
                                text_color,
                            );
                        }
                    }
                });
            });

        bar_height
    }
}
