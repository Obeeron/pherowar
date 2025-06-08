// Timer struct for countdown logic in simulation
// Counts up from 0 to max_value

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

    /// Returns true if the timer has gone past the max value
    pub fn is_ready(&self) -> bool {
        self.value >= self.max_value
    }

    /// Update the timer by dt (delta time)
    pub fn update(&mut self, dt: f32) {
        self.value += dt;
    }

    /// Wraps the timer value back within bounds.
    pub fn wrap(&mut self) {
        self.value %= self.max_value;
    }

    /// Force the timer to be ready
    pub fn force_ready(&mut self) {
        self.value = self.max_value;
    }
}
