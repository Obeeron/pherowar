use bincode_derive::{Decode, Encode};
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};

use super::MAX_PHEROMONE_AMOUNT;

#[derive(Encode, Decode, Clone, Serialize, Deserialize)]
pub struct PheromoneChannel {
    pub width: u32,
    pub height: u32,
    pub data: Vec<Vec<f32>>,
    pub decay_rate: f32,
}

impl PheromoneChannel {
    pub fn new(width: u32, height: u32, decay_rate: f32) -> Self {
        Self {
            width,
            height,
            data: vec![vec![0.0; width as usize]; height as usize],
            decay_rate,
        }
    }

    #[inline(always)]
    pub fn lay(&mut self, x: usize, y: usize, amount: f32) {
        let cell = &mut self.data[y][x];
        *cell = (*cell + amount).min(MAX_PHEROMONE_AMOUNT);
    }

    pub fn decay(&mut self) {
        let width = self.width as usize;
        let height = self.height as usize;
        for y in 0..height {
            for x in 0..width {
                if self.data[y][x] > 0.0 {
                    self.data[y][x] *= self.decay_rate;
                }
                if self.data[y][x] < 0.01 {
                    self.data[y][x] = 0.0;
                }
            }
        }
    }
}
