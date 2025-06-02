use new_egui_macroquad::egui;

use crate::editor::EditorManager;
use crate::ui::events::UIEvent;

pub struct ToolSizeSlider;

impl ToolSizeSlider {
    pub fn new() -> Self {
        Self
    }

    pub fn draw(&self, ui: &mut egui::Ui, editor: &mut EditorManager) -> Option<UIEvent> {
        let mut event = None;

        ui.vertical(|ui| {
            ui.label(egui::RichText::new("Tool Size").strong());

            let mut size = editor.tool_size();
            let slider = ui.add(
                egui::Slider::new(&mut size, 1.0..=100.0)
                    .show_value(true)
                    .fixed_decimals(0)
                    .clamp_to_range(true)
                    .text("px"),
            );

            if slider.changed() {
                editor.set_tool_size(size);
                event = Some(UIEvent::ToolSizeChanged(size));
            }
        });

        event
    }
}
