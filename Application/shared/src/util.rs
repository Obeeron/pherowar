use once_cell::sync::Lazy;
use std::f32;

const LOOKUP_TABLE_SIZE: usize = 360;

/// Precomputed sine and cosine values for equally spaced angles around the circle.
static SIN_COS_TABLE: Lazy<[(f32, f32); LOOKUP_TABLE_SIZE]> = Lazy::new(|| {
    let mut arr = [(0.0f32, 0.0f32); LOOKUP_TABLE_SIZE];
    let step = std::f32::consts::TAU / LOOKUP_TABLE_SIZE as f32;
    for i in 0..LOOKUP_TABLE_SIZE {
        let angle = i as f32 * step;
        arr[i] = (angle.sin(), angle.cos());
    }
    arr
});

/// Fast sine and cosine using lookup table. Angle normalized via rem_euclid.
#[inline(always)]
pub fn fast_sin_cos(angle: f32) -> (f32, f32) {
    let frac = angle.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU;
    let idx = ((frac * LOOKUP_TABLE_SIZE as f32) as usize) % LOOKUP_TABLE_SIZE;
    SIN_COS_TABLE[idx]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    #[test]
    fn test_fast_sin_cos_accuracy() {
        let angles = [0.0, PI / 4.0, PI / 2.0, PI, 3.0 * PI / 2.0, 2.0 * PI];
        for &angle in &angles {
            let (fast_sin, fast_cos) = fast_sin_cos(angle);
            let (true_sin, true_cos) = ((angle as f32).sin(), (angle as f32).cos());

            assert!(
                (fast_sin - true_sin).abs() < 0.01,
                "Sin value inaccurate for angle: {}",
                angle
            );
            assert!(
                (fast_cos - true_cos).abs() < 0.01,
                "Cos value inaccurate for angle: {}",
                angle
            );
        }
    }

    #[test]
    fn test_fast_sin_cos_wraparound() {
        let angle = 2.5 * PI; // Beyond 2π
        let (fast_sin, fast_cos) = fast_sin_cos(angle);
        let (true_sin, true_cos) = (
            ((angle % (2.0 * PI)) as f32).sin(),
            ((angle % (2.0 * PI)) as f32).cos(),
        );

        assert!(
            (fast_sin - true_sin).abs() < 0.01,
            "Sin value inaccurate for wrapped angle: {}",
            angle
        );
        assert!(
            (fast_cos - true_cos).abs() < 0.01,
            "Cos value inaccurate for wrapped angle: {}",
            angle
        );
    }

    #[test]
    fn test_fast_sin_cos_negative_angles() {
        let angle = -PI / 2.0; // Negative angle
        let (fast_sin, fast_cos) = fast_sin_cos(angle);
        let (true_sin, true_cos) = ((angle as f32).sin(), (angle as f32).cos());

        assert!(
            (fast_sin - true_sin).abs() < 0.01,
            "Sin value inaccurate for negative angle: {}",
            angle
        );
        assert!(
            (fast_cos - true_cos).abs() < 0.01,
            "Cos value inaccurate for negative angle: {}",
            angle
        );
    }

    #[test]
    fn test_fast_sin_cos_zero_angle() {
        let angle = 0.0;
        let (fast_sin, fast_cos) = fast_sin_cos(angle);
        assert!(
            (fast_sin - 0.0).abs() < 0.01,
            "Sin value inaccurate for zero angle"
        );
        assert!(
            (fast_cos - 1.0).abs() < 0.01,
            "Cos value inaccurate for zero angle"
        );
    }

    #[test]
    fn test_fast_sin_cos_full_circle() {
        let angle = 2.0 * PI; // Full circle
        let (fast_sin, fast_cos) = fast_sin_cos(angle);
        assert!(
            (fast_sin - 0.0).abs() < 0.01,
            "Sin value inaccurate for full circle"
        );
        assert!(
            (fast_cos - 1.0).abs() < 0.01,
            "Cos value inaccurate for full circle"
        );
    }

    #[test]
    fn test_fast_sin_cos_large_angle() {
        let angle = 10.0 * PI; // Large angle
        let (fast_sin, fast_cos) = fast_sin_cos(angle);
        let (true_sin, true_cos) = (
            ((angle % (2.0 * PI)) as f32).sin(),
            ((angle % (2.0 * PI)) as f32).cos(),
        );
        assert!(
            (fast_sin - true_sin).abs() < 0.01,
            "Sin value inaccurate for large angle"
        );
        assert!(
            (fast_cos - true_cos).abs() < 0.01,
            "Cos value inaccurate for large angle"
        );
    }

    #[test]
    fn test_fast_sin_cos_small_angle() {
        let angle = 0.0001; // Very small angle
        let (fast_sin, fast_cos) = fast_sin_cos(angle);
        let (true_sin, true_cos) = ((angle as f32).sin(), (angle as f32).cos());
        assert!(
            (fast_sin - true_sin).abs() < 0.01,
            "Sin value inaccurate for small angle"
        );
        assert!(
            (fast_cos - true_cos).abs() < 0.01,
            "Cos value inaccurate for small angle"
        );
    }
}
