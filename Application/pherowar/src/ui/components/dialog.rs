use new_egui_macroquad::egui;

/// DialogPopupMode determines if the dialog is for confirmation or text input
pub enum DialogPopupMode {
    Confirm { message: String },
    Input { label: String, value: String },
    Info { message: String },
    NewMap { width: u32, height: u32 },
}

pub enum DialogPopupResult {
    Confirmed,
    Cancelled,
    InputConfirmed(String),
    InfoOk, // Result for info dialog
    /// Result for new map dialog
    NewMapConfirmed {
        width: u32,
        height: u32,
    },
}

pub struct DialogPopup {
    pub open: bool,
    pub mode: DialogPopupMode,
    pub result: Option<DialogPopupResult>,
    pub options: Option<Vec<String>>, // Generic selectable options for any dialog
}

impl DialogPopup {
    pub fn new_confirm(message: &str) -> Self {
        Self {
            open: true,
            mode: DialogPopupMode::Confirm {
                message: message.to_string(),
            },
            result: None,
            options: None,
        }
    }

    pub fn new_input(label: &str, default: &str) -> Self {
        Self {
            open: true,
            mode: DialogPopupMode::Input {
                label: label.to_string(),
                value: default.to_string(),
            },
            result: None,
            options: None,
        }
    }

    pub fn new_map_picker(options: Vec<String>) -> Self {
        let value = options.get(0).cloned().unwrap_or_default();
        Self {
            open: true,
            mode: DialogPopupMode::Input {
                label: "Select map to load:".to_string(),
                value,
            },
            result: None,
            options: Some(options),
        }
    }

    pub fn new_info(message: &str) -> Self {
        Self {
            open: true,
            mode: DialogPopupMode::Info {
                message: message.to_string(),
            },
            result: None,
            options: None,
        }
    }

    /// New map dialog constructor
    pub fn new_new_map(default_width: u32, default_height: u32) -> Self {
        Self {
            open: true,
            mode: DialogPopupMode::NewMap {
                width: default_width,
                height: default_height,
            },
            result: None,
            options: None,
        }
    }

    /// Draw the dialog. Returns true if dialog is still open, false if closed.
    pub fn draw(&mut self, egui_ctx: &egui::Context) -> bool {
        if !self.open {
            return false;
        }
        let mut still_open = true;
        // Draw modal overlay
        egui::Area::new("modal_overlay".into())
            .order(egui::Order::Background)
            .show(egui_ctx, |ui| {
                let screen_rect = egui_ctx.screen_rect();
                let overlay_color = egui::Color32::from_rgba_premultiplied(20, 20, 20, 180);
                ui.painter().rect_filled(screen_rect, 0.0, overlay_color);
            });
        egui::Window::new("")
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(egui_ctx, |ui| match &mut self.mode {
                DialogPopupMode::Confirm { message } => {
                    ui.label(message.as_str());
                    ui.horizontal(|ui| {
                        if ui.button("Confirm").clicked() {
                            self.result = Some(DialogPopupResult::Confirmed);
                            self.open = false;
                            still_open = false;
                        }
                        if ui.button("Cancel").clicked() {
                            self.result = Some(DialogPopupResult::Cancelled);
                            self.open = false;
                            still_open = false;
                        }
                    });
                }
                DialogPopupMode::Input { label, value } => {
                    if label == "Select map to load:" {
                        if let Some(options) = &self.options {
                            let selected = value.clone();
                            egui::ComboBox::from_id_source("dialog_picker_combo")
                                .selected_text(selected.clone())
                                .show_ui(ui, |ui| {
                                    for opt in options {
                                        if ui.selectable_label(&selected == opt, opt).clicked() {
                                            *value = opt.clone();
                                        }
                                    }
                                });
                        } else {
                            ui.label(label.as_str());
                            ui.text_edit_singleline(value);
                        }
                    } else {
                        ui.label(label.as_str());
                        let response = ui.text_edit_singleline(value);
                        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            self.result = Some(DialogPopupResult::InputConfirmed(value.clone()));
                            self.open = false;
                            still_open = false;
                        }
                    }
                    ui.horizontal(|ui| {
                        let is_save = label.to_lowercase().contains("save");
                        let main_button = if is_save { "Save" } else { "Load" };
                        if ui.button(main_button).clicked()
                            || (label == "Select map to load:"
                                && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                        {
                            self.result = Some(DialogPopupResult::InputConfirmed(value.clone()));
                            self.open = false;
                            still_open = false;
                        }
                        if ui.button("Cancel").clicked() {
                            self.result = Some(DialogPopupResult::Cancelled);
                            self.open = false;
                            still_open = false;
                        }
                    });
                }
                DialogPopupMode::Info { message } => {
                    ui.label(message.as_str());
                    ui.add_space(8.0);
                    if ui.button("Ok").clicked() {
                        self.result = Some(DialogPopupResult::InfoOk);
                        self.open = false;
                        still_open = false;
                    }
                }
                DialogPopupMode::NewMap { width, height } => {
                    ui.label("Create New Map");
                    ui.horizontal(|ui| {
                        ui.label("Width:");
                        let mut w = *width as i32;
                        if ui
                            .add(egui::DragValue::new(&mut w).range(16..=4096))
                            .changed()
                        {
                            *width = w.max(16) as u32;
                        }
                        ui.label("Height:");
                        let mut h = *height as i32;
                        if ui
                            .add(egui::DragValue::new(&mut h).range(16..=4096))
                            .changed()
                        {
                            *height = h.max(16) as u32;
                        }
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Create").clicked() {
                            self.result = Some(DialogPopupResult::NewMapConfirmed {
                                width: *width,
                                height: *height,
                            });
                            self.open = false;
                            still_open = false;
                        }
                        if ui.button("Cancel").clicked() {
                            self.result = Some(DialogPopupResult::Cancelled);
                            self.open = false;
                            still_open = false;
                        }
                    });
                }
            });
        still_open
    }
}
