use macroquad::prelude::*;

// Camera configuration constants
/// Minimum zoom level allowed (1.0 = full map view)
const MIN_ZOOM: f32 = 1.0;
/// Maximum zoom level allowed
const MAX_ZOOM: f32 = 50.0;
/// Speed multiplier for zoom operations
const ZOOM_SPEED: f32 = 0.1;

/// A camera system for 2D game worlds that handles zooming and panning
pub struct GameCamera {
    /// Zoom level (minimum 1.0, higher values zoom in)
    zoom: f32,

    /// Map dimensions
    pub map_width: u32,
    pub map_height: u32,

    /// The actual macroquad camera object
    pub camera: Camera2D,
}

impl GameCamera {
    /// Creates a new camera system for the given map dimensions
    pub fn new(map_width: u32, map_height: u32) -> Self {
        let mut camera = Self {
            zoom: 1.0,
            map_width,
            map_height,
            camera: Camera2D {
                target: vec2(map_width as f32 / 2.0, map_height as f32 / 2.0),
                ..Default::default()
            },
        };

        // Initialize zoom
        camera.update_camera_zoom();

        camera
    }

    pub fn adjust_zoom(&mut self, wheel_movement: f32) {
        let old_zoom = self.zoom;

        // Store mouse position and convert to world coordinates before zoom change
        let mouse_screen_pos = Vec2::from(mouse_position());
        let mouse_world_pos = self.camera.screen_to_world(mouse_screen_pos);

        // Adjust zoom level
        self.zoom = (self.zoom - wheel_movement * self.zoom * ZOOM_SPEED).clamp(MIN_ZOOM, MAX_ZOOM);

        // If zoom level changed, update camera parameters
        if old_zoom != self.zoom {
            // Update the camera zoom values
            self.update_camera_zoom();

            // Get the new position of the same world point after zoom
            let new_mouse_world_pos = self.camera.screen_to_world(mouse_screen_pos);

            // Move the camera to keep the point under cursor
            let position_delta = mouse_world_pos - new_mouse_world_pos;
            self.move_by(position_delta);
        }
    }

    pub fn move_by(&mut self, movement: Vec2) {
        self.camera.target += movement;
        self.adjust_camera_bounds();
    }

    fn update_camera_zoom(&mut self) {
        let map_ratio = self.map_width as f32 / self.map_height as f32;
        let screen_ratio = screen_width() / screen_height();

        // Calculate aspect ratio adjustments to prevent distortion
        let (horizontal_adjustment, vertical_adjustment) = if map_ratio >= screen_ratio {
            // Map is wider than screen, adjust horizontal zoom
            (map_ratio / screen_ratio, 1.0)
        } else {
            // Map is taller than screen, adjust vertical zoom
            (1.0, screen_ratio / map_ratio)
        };

        self.camera.zoom = vec2(
            1.0 / self.map_width as f32 * 2.0 * self.zoom * horizontal_adjustment,
            1.0 / self.map_height as f32 * 2.0 * self.zoom * vertical_adjustment,
        );
    }

    // Helper method to keep camera within map bounds
    fn adjust_camera_bounds(&mut self) {
        // Calculate view dimensions based on zoom level
        let map_ratio = self.map_width as f32 / self.map_height as f32;
        let screen_ratio = screen_width() / screen_height();

        // Apply the same aspect ratio adjustments as in update_camera_zoom
        let horizontal_view = if map_ratio >= screen_ratio {
            (self.map_width as f32 / self.zoom) * (screen_ratio / map_ratio)
        } else {
            self.map_width as f32 / self.zoom
        };

        let vertical_view = if map_ratio >= screen_ratio {
            self.map_height as f32 / self.zoom
        } else {
            (self.map_height as f32 / self.zoom) * (map_ratio / screen_ratio)
        };

        // Adjust X coordinate
        self.camera.target.x =
            self.adjust_coordinate(self.camera.target.x, horizontal_view, self.map_width as f32);

        // Adjust Y coordinate
        self.camera.target.y =
            self.adjust_coordinate(self.camera.target.y, vertical_view, self.map_height as f32);
    }

    // Helper to adjust a single coordinate (x or y)
    fn adjust_coordinate(&self, value: f32, view_size: f32, map_size: f32) -> f32 {
        let min = view_size / 2.0;
        let max = map_size - min;

        if max < min {
            // View is larger than map, center the camera
            map_size / 2.0
        } else {
            // Clamp within bounds
            value.clamp(min, max)
        }
    }

    /// Converts the current mouse screen position to world coordinates
    pub fn get_mouse_world_pos(&self) -> Vec2 {
        self.camera.screen_to_world(Vec2::from(mouse_position()))
    }

    /// Sets the camera target to a specific world position.
    pub fn set_target(&mut self, target_pos: Vec2) {
        self.camera.target = target_pos;
        self.adjust_camera_bounds(); // Ensure the new target is within bounds
    }

    /// Resets the camera to its default position and zoom
    pub fn reset(&mut self) {
        self.zoom = 1.0;
        self.camera.target = vec2(self.map_width as f32 / 2.0, self.map_height as f32 / 2.0);
        self.update_camera_zoom();
        // Ensure bounds are correct after reset
        self.adjust_camera_bounds();
    }

    /// Handles window resize events.
    pub fn handle_resize(&mut self) {
        // Recalculate zoom based on new screen dimensions
        self.update_camera_zoom();
        // Immediately enforce boundaries with the new zoom/dimensions
        self.adjust_camera_bounds();
    }
}
