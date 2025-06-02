use crate::simulation::Simulation;
use macroquad::prelude::Color;

pub const PREDEFINED_COLONY_COLORS: [Color; 5] = [
    Color::new(0.902, 0.224, 0.275, 1.0), // Red
    Color::new(0.169, 0.635, 0.929, 1.0), // Blue
    Color::new(0.149, 0.878, 0.184, 1.0), // Green
    Color::new(0.957, 0.820, 0.204, 1.0), // Yellow
    Color::new(0.616, 0.306, 0.867, 1.0), // Purple
];

/// Manages selection of colony colors from a predefined palette.
pub struct ColorPalette {
    selected_index: usize, // Index of the currently selected color in PREDEFINED_COLONY_COLORS
}

impl ColorPalette {
    /// Creates a new `ColorPalette`, selecting the first color by default.
    pub fn new() -> Self {
        Self { selected_index: 0 }
    }

    /// Gets the currently selected `Color`.
    pub fn get_selected_color(&self) -> Color {
        PREDEFINED_COLONY_COLORS[self.selected_index]
    }

    /// Gets the index of the currently selected color.
    pub fn get_selected_index(&self) -> usize {
        self.selected_index
    }

    /// Sets the selected color index, ensuring it's within bounds.
    pub fn set_selected_index(&mut self, index: usize) {
        if index < PREDEFINED_COLONY_COLORS.len() {
            self.selected_index = index;
        } else {
            eprintln!(
                "Attempted to set invalid color index: {} (max is {})",
                index,
                PREDEFINED_COLONY_COLORS.len() - 1
            );
            // Keeps current index if out of bounds
        }
    }

    /// Checks if two colors are approximately equal (within EPSILON).
    fn colors_are_close(c1: Color, c2: Color) -> bool {
        const EPSILON: f32 = 0.01;
        (c1.r - c2.r).abs() < EPSILON
            && (c1.g - c2.g).abs() < EPSILON
            && (c1.b - c2.b).abs() < EPSILON
        // Alpha is not compared for now
    }

    /// Returns a list of colors currently used by active colonies.
    fn get_used_colors(simulation: &Simulation) -> Vec<Color> {
        simulation.colonies.values().map(|c| c.color).collect()
    }

    /// Checks if a specific `color` is currently used by any colony.
    pub fn is_color_used(color: Color, simulation: &Simulation) -> bool {
        Self::get_used_colors(simulation)
            .iter()
            .any(|&used_color| Self::colors_are_close(color, used_color))
    }

    /// Checks if all predefined colors are currently in use by colonies.
    pub fn are_all_colors_used(simulation: &Simulation) -> bool {
        let used_colors = Self::get_used_colors(simulation);
        // True if number of unique used colors is at least the number of predefined colors.
        // This simple check assumes predefined colors are distinct and used colors are from this set.
        used_colors.len() >= PREDEFINED_COLONY_COLORS.len()
        // A more robust check (if external colors or duplicates were possible):
        // PREDEFINED_COLONY_COLORS.iter().all(|&palette_color| {
        //     used_colors.iter().any(|&used_color| Self::colors_are_close(palette_color, used_color))
        // })
    }

    /// Updates selected color to the first available one if current is used.
    /// Returns `true` if selection changed, `false` otherwise.
    pub fn update_selection(&mut self, simulation: &Simulation) -> bool {
        let used_colors = Self::get_used_colors(simulation);

        // If current selection is available, no change needed.
        let current_color = self.get_selected_color();
        let current_is_used = used_colors
            .iter()
            .any(|&used| Self::colors_are_close(current_color, used));

        if !current_is_used {
            return false;
        }

        // Current selection is used, find the first available alternative.
        for (idx, candidate_color) in PREDEFINED_COLONY_COLORS.iter().enumerate() {
            let is_candidate_used = used_colors
                .iter()
                .any(|&used| Self::colors_are_close(*candidate_color, used));

            if !is_candidate_used {
                if self.selected_index != idx {
                    self.selected_index = idx;
                    return true; // New available color selected
                } else {
                    // This should not be reached if current_is_used is true and this is the current_color.
                    return false; // Defensive: current is available (somehow)
                }
            }
        }
        // All colors are used, or no alternative found; keep current selection.
        false
    }
}

impl Default for ColorPalette {
    fn default() -> Self {
        Self::new()
    }
}
