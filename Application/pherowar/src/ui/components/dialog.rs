use new_egui_macroquad::egui;

/// Dialog purpose - what action should be taken when confirmed
#[derive(Debug, Clone)]
pub enum DialogPurpose {
    Info,
    Confirmation,
    NewMap,
    LoadMap,
    SaveMap,
}

/// Dialog content types
#[derive(Debug, Clone)]
pub enum DialogContent {
    Message(String),
    Input {
        label: String,
        value: String,
    },
    TwoNumbers {
        label1: String,
        label2: String,
        value1: f64,
        value2: f64,
        min1: Option<f64>,
        max1: Option<f64>,
        min2: Option<f64>,
        max2: Option<f64>,
    },
    Choice {
        label: String,
        options: Vec<String>,
        selected: usize,
    },
}

/// Dialog result types
#[derive(Debug, Clone)]
pub enum DialogResult {
    Confirmed,
    Cancelled,
    InputConfirmed,
    TwoNumberConfirmed(f64, f64),
    ChoiceConfirmed(String),
}

/// Main dialog popup struct
pub struct DialogPopup {
    pub open: bool,
    pub title: Option<String>,
    pub purpose: DialogPurpose,
    pub content: DialogContent,
    pub result: Option<DialogResult>,
}

impl DialogPopup {
    pub fn new_confirm(message: &str) -> Self {
        Self {
            open: true,
            title: None,
            purpose: DialogPurpose::Confirmation,
            content: DialogContent::Message(message.to_string()),
            result: None,
        }
    }

    /// Create an info dialog with a title
    pub fn new_info_with_title(title: &str, message: &str) -> Self {
        Self {
            open: true,
            title: Some(title.to_string()),
            purpose: DialogPurpose::Info,
            content: DialogContent::Message(message.to_string()),
            result: None,
        }
    }

    /// Create a save map input dialog
    pub fn new_save_map_input(prefill_name: &str) -> Self {
        Self {
            open: true,
            title: Some("Save Map".to_string()),
            purpose: DialogPurpose::SaveMap,
            content: DialogContent::Input {
                label: "Enter map name to save:".to_string(),
                value: prefill_name.to_string(),
            },
            result: None,
        }
    }

    pub fn new_map_picker(options: Vec<String>) -> Self {
        let selected = 0;
        Self {
            open: true,
            title: Some("Load Map".to_string()),
            purpose: DialogPurpose::LoadMap,
            content: DialogContent::Choice {
                label: "Select map to load:".to_string(),
                options,
                selected,
            },
            result: None,
        }
    }

    pub fn new_info(message: &str) -> Self {
        Self {
            open: true,
            title: None,
            purpose: DialogPurpose::Info,
            content: DialogContent::Message(message.to_string()),
            result: None,
        }
    }

    /// New map dialog constructor
    pub fn new_new_map(default_width: u32, default_height: u32) -> Self {
        Self {
            open: true,
            title: Some("Create New Map".to_string()),
            purpose: DialogPurpose::NewMap,
            content: DialogContent::TwoNumbers {
                label1: "Width:".to_string(),
                label2: "Height:".to_string(),
                value1: default_width as f64,
                value2: default_height as f64,
                min1: Some(16.0),
                max1: Some(4096.0),
                min2: Some(16.0),
                max2: Some(4096.0),
            },
            result: None,
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

        let window_title = self.title.as_deref().unwrap_or("");
        egui::Window::new(window_title)
            .title_bar(self.title.is_some())
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .min_width(300.0)
            .min_height(120.0)
            .show(egui_ctx, |ui| {
                match &mut self.content {
                    DialogContent::Message(message) => {
                        ui.label(message.as_str());
                        ui.add_space(8.0);

                        // Handle keyboard input for confirmation dialogs
                        if matches!(self.purpose, DialogPurpose::Confirmation) {
                            if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                self.result = Some(DialogResult::Confirmed);
                                self.open = false;
                                still_open = false;
                            } else if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                                self.result = Some(DialogResult::Cancelled);
                                self.open = false;
                                still_open = false;
                            }
                        }

                        ui.horizontal(|ui| match self.purpose {
                            DialogPurpose::Confirmation => {
                                if ui.button("Confirm").clicked() {
                                    self.result = Some(DialogResult::Confirmed);
                                    self.open = false;
                                    still_open = false;
                                }
                                if ui.button("Cancel").clicked() {
                                    self.result = Some(DialogResult::Cancelled);
                                    self.open = false;
                                    still_open = false;
                                }
                            }
                            _ => {
                                if ui.button("Ok").clicked() {
                                    self.result = Some(DialogResult::Confirmed);
                                    self.open = false;
                                    still_open = false;
                                }
                            }
                        });
                    }
                    DialogContent::Input { label, value } => {
                        ui.label(label.as_str());
                        let response = ui.text_edit_singleline(value);

                        // Handle keyboard input
                        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            self.result = Some(DialogResult::InputConfirmed);
                            self.open = false;
                            still_open = false;
                        } else if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                            self.result = Some(DialogResult::Cancelled);
                            self.open = false;
                            still_open = false;
                        }

                        ui.horizontal(|ui| {
                            let button_text = match self.purpose {
                                DialogPurpose::SaveMap => "Save",
                                DialogPurpose::LoadMap => "Load",
                                _ => "Ok",
                            };
                            if ui.button(button_text).clicked() {
                                self.result = Some(DialogResult::InputConfirmed);
                                self.open = false;
                                still_open = false;
                            }
                            if ui.button("Cancel").clicked() {
                                self.result = Some(DialogResult::Cancelled);
                                self.open = false;
                                still_open = false;
                            }
                        });
                    }
                    DialogContent::TwoNumbers {
                        label1,
                        label2,
                        value1,
                        value2,
                        min1,
                        max1,
                        min2,
                        max2,
                    } => {
                        ui.horizontal(|ui| {
                            ui.label(label1.as_str());
                            let mut v1 = *value1 as i32;
                            let range1 =
                                min1.unwrap_or(0.0) as i32..=max1.unwrap_or(10000.0) as i32;
                            if ui
                                .add(egui::DragValue::new(&mut v1).range(range1))
                                .changed()
                            {
                                *value1 = v1.max(min1.unwrap_or(0.0) as i32) as f64;
                            }

                            ui.label(label2.as_str());
                            let mut v2 = *value2 as i32;
                            let range2 =
                                min2.unwrap_or(0.0) as i32..=max2.unwrap_or(10000.0) as i32;
                            if ui
                                .add(egui::DragValue::new(&mut v2).range(range2))
                                .changed()
                            {
                                *value2 = v2.max(min2.unwrap_or(0.0) as i32) as f64;
                            }
                        });

                        // Handle keyboard input
                        if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            self.result = Some(DialogResult::TwoNumberConfirmed(*value1, *value2));
                            self.open = false;
                            still_open = false;
                        } else if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                            self.result = Some(DialogResult::Cancelled);
                            self.open = false;
                            still_open = false;
                        }

                        ui.horizontal(|ui| {
                            if ui.button("Create").clicked() {
                                self.result =
                                    Some(DialogResult::TwoNumberConfirmed(*value1, *value2));
                                self.open = false;
                                still_open = false;
                            }
                            if ui.button("Cancel").clicked() {
                                self.result = Some(DialogResult::Cancelled);
                                self.open = false;
                                still_open = false;
                            }
                        });
                    }
                    DialogContent::Choice {
                        label,
                        options,
                        selected,
                    } => {
                        ui.label(label.as_str());
                        let current_option = options.get(*selected).cloned().unwrap_or_default();
                        egui::ComboBox::from_id_source("dialog_picker_combo")
                            .selected_text(current_option.clone())
                            .show_ui(ui, |ui| {
                                for (i, opt) in options.iter().enumerate() {
                                    if ui.selectable_label(*selected == i, opt).clicked() {
                                        *selected = i;
                                    }
                                }
                            });

                        // Handle keyboard input
                        if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            let selected_option =
                                options.get(*selected).cloned().unwrap_or_default();
                            self.result = Some(DialogResult::ChoiceConfirmed(selected_option));
                            self.open = false;
                            still_open = false;
                        } else if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                            self.result = Some(DialogResult::Cancelled);
                            self.open = false;
                            still_open = false;
                        }

                        ui.horizontal(|ui| {
                            if ui.button("Load").clicked() {
                                let selected_option =
                                    options.get(*selected).cloned().unwrap_or_default();
                                self.result = Some(DialogResult::ChoiceConfirmed(selected_option));
                                self.open = false;
                                still_open = false;
                            }
                            if ui.button("Cancel").clicked() {
                                self.result = Some(DialogResult::Cancelled);
                                self.open = false;
                                still_open = false;
                            }
                        });
                    }
                }
            });
        still_open
    }
}
