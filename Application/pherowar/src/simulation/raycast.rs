// use crate::simulation::map::Terrain; // This will be effectively replaced by the is_wall_fn logic
use shared::fast_sin_cos; // Assuming this provides `fn fast_sin_cos(angle: f32) -> (f32, f32)`
use std::f32::consts::{PI, TAU};

// Import the constant from the parent module to ensure consistency
use super::SENSE_MAX_DISTANCE;

/// Number of discrete rays so that adjacent rays at max distance are ≤1 cell apart.
pub const ANGLE_COUNT: usize = (2.0 * PI * SENSE_MAX_DISTANCE) as usize;

/// A simple, flat cache for raycast distances.
/// Stores one f32 per (x,y,ray):
///   NaN = uncomputed or invalidated,
///   ∞ = no wall hit.
pub struct RaycastCache {
    width: usize,    // Should match map width
    height: usize,   // Should match map height
    cache: Vec<f32>, // [x][y][ray]
}

impl RaycastCache {
    pub fn new(width: usize, height: usize) -> Self {
        RaycastCache {
            width,
            height,
            cache: vec![f32::NAN; width * height * ANGLE_COUNT],
        }
    }

    /// Clear all cached values to NaN (needs recomputation)
    pub fn clear(&mut self) {
        self.cache.fill(f32::NAN);
    }

    /// Clear cached raycast results for a specific position and surrounding area to NaN
    /// This should be called when terrain changes to invalidate affected cache entries
    pub fn invalidate_area_around(&mut self, center_x: usize, center_y: usize) {
        // SENSE_MAX_DISTANCE is already imported at the top of the file
        let radius = SENSE_MAX_DISTANCE.ceil() as usize + 1;

        let min_x = center_x.saturating_sub(radius);
        let max_x = (center_x + radius + 1).min(self.width);
        let min_y = center_y.saturating_sub(radius);
        let max_y = (center_y + radius + 1).min(self.height);

        for y in min_y..max_y {
            for x in min_x..max_x {
                self.invalidate_cell(x, y);
            }
        }
    }

    /// Clear cached raycast results for a specific cell only to NaN
    pub fn invalidate_cell(&mut self, x: usize, y: usize) {
        if x < self.width && y < self.height {
            for ray in 0..ANGLE_COUNT {
                let index = self.idx(x, y, ray);
                self.cache[index] = f32::NAN;
            }
        }
    }

    /// Compute flat cache index.
    #[inline]
    fn idx(&self, x: usize, y: usize, ray: usize) -> usize {
        (y * self.width + x) * ANGLE_COUNT + ray
    }

    /// Map a continuous angle [0,2π) to a discrete ray index.
    pub fn angle_to_ray_index(angle: f32) -> usize {
        (((angle.rem_euclid(TAU) / TAU) * ANGLE_COUNT as f32).round() as usize) % ANGLE_COUNT
    }

    /// Helper to convert ray index back to an angle in degrees for display.
    pub fn ray_index_to_angle(ray_idx: usize) -> f32 {
        (ray_idx as f32 / ANGLE_COUNT as f32) * TAU
    }

    /// If the result is not cached (NaN), it will be computed and cached.
    pub fn get_distance_at_angle<F>(
        &mut self,
        is_wall_fn: &F,
        x: usize,
        y: usize,
        angle: f32,
    ) -> Option<f32>
    where
        F: Fn(usize, usize) -> bool,
    {
        if x >= self.width || y >= self.height {
            return None; // Origin is outside cache dimensions
        }
        let ray_idx = Self::angle_to_ray_index(angle);
        let cache_flat_idx = self.idx(x, y, ray_idx);

        if self.cache[cache_flat_idx].is_nan() {
            self.compute_single_ray(is_wall_fn, x, y, ray_idx);
        }
        Some(self.cache[cache_flat_idx])
    }

    fn compute_single_ray<F>(&mut self, is_wall_fn: &F, x: usize, y: usize, ray_idx: usize)
    where
        F: Fn(usize, usize) -> bool,
    {
        let ox = x as f32 + 0.5;
        let oy = y as f32 + 0.5;
        let theta = Self::ray_index_to_angle(ray_idx);
        let (sin_a, cos_a) = fast_sin_cos(theta);

        let mut current_hit_dist = f32::INFINITY;

        let mut current_map_x = x as isize;
        let mut current_map_y = y as isize;

        let step_x: isize = if cos_a > 0.0 { 1 } else { -1 };
        let step_y: isize = if sin_a > 0.0 { 1 } else { -1 };

        let t_delta_x = if cos_a.abs() < 1e-6 {
            f32::INFINITY
        } else {
            (1.0 / cos_a).abs()
        };
        let t_delta_y = if sin_a.abs() < 1e-6 {
            f32::INFINITY
        } else {
            (1.0 / sin_a).abs()
        };

        let mut t_max_x = if cos_a.abs() < 1e-6 {
            f32::INFINITY
        } else if cos_a > 0.0 {
            ((x as f32 + 1.0) - ox) / cos_a
        } else {
            (x as f32 - ox) / cos_a
        };

        let mut t_max_y = if sin_a.abs() < 1e-6 {
            f32::INFINITY
        } else if sin_a > 0.0 {
            ((y as f32 + 1.0) - oy) / sin_a
        } else {
            (y as f32 - oy) / sin_a
        };

        loop {
            let dist_to_boundary;

            if t_max_x < t_max_y {
                if t_max_x > SENSE_MAX_DISTANCE {
                    break;
                }
                dist_to_boundary = t_max_x;
                current_map_x += step_x;
                t_max_x += t_delta_x;
            } else {
                if t_max_y > SENSE_MAX_DISTANCE {
                    break;
                }
                dist_to_boundary = t_max_y;
                current_map_y += step_y;
                t_max_y += t_delta_y;
            }

            if current_map_x < 0
                || current_map_x as usize >= self.width
                || current_map_y < 0
                || current_map_y as usize >= self.height
            {
                break;
            }

            if is_wall_fn(current_map_x as usize, current_map_y as usize) {
                current_hit_dist = dist_to_boundary;
                break;
            }
        }
        let cache_idx = self.idx(x, y, ray_idx); // Calculate index before mutable borrow
        self.cache[cache_idx] = current_hit_dist;
    }

    /// Recompute the cache for all rays in (x,y) using DDA.
    /// This is kept for cases where recomputing all rays for a cell is beneficial,
    /// e.g. after a major change or for debugging.
    pub fn recompute_all_rays_for_cell<F>(&mut self, is_wall_fn: &F, x: usize, y: usize)
    where
        F: Fn(usize, usize) -> bool,
    {
        if x >= self.width || y >= self.height {
            return;
        }
        for ray_idx in 0..ANGLE_COUNT {
            self.compute_single_ray(is_wall_fn, x, y, ray_idx);
        }
    }

    pub fn recompute_all_cache<F>(&mut self, is_wall_fn: &F)
    where
        F: Fn(usize, usize) -> bool,
    {
        for y_coord in 0..self.height {
            for x_coord in 0..self.width {
                // Don't compute rays *from* a cell that is currently a wall.
                if is_wall_fn(x_coord, y_coord) {
                    // Optionally, ensure cache for wall cells is marked (e.g. NaN or specific value)
                    // Current invalidation logic should handle this when walls change.
                    // For now, just skip computation.
                    // If a wall cell becomes non-wall, its cache entries (NaN) will trigger recompute on access.
                    continue;
                }
                self.recompute_all_rays_for_cell(is_wall_fn, x_coord, y_coord);
            }
        }
    }
}
