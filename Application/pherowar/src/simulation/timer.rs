// Timer struct for countdown logic in simulation
// Counts down from initial_value to 0, then can be reset to max_value

#[derive(Debug, Clone)]
pub struct Timer {
    pub max_value: f32,
    pub value: f32,
}

impl Timer {
    /// Create a new timer with a max value and an initial value
    pub fn new(max_value: f32, initial_value: f32) -> Self {
        Self {
            max_value,
            value: initial_value,
        }
    }

    /// Returns true if the timer has reached zero or below
    pub fn is_ready(&self) -> bool {
        self.value <= 0.0
    }

    /// Decrease the timer by dt (delta time)
    pub fn update(&mut self, dt: f32) {
        self.value -= dt;
    }

    /// Reset the timer to max_value
    pub fn reset(&mut self) {
        self.value = self.max_value;
    }

    /// Force the timer to be ready
    pub fn force_ready(&mut self) {
        self.value = 0.0;
    }
}
