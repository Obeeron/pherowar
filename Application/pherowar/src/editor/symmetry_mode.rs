// Manages symmetry modes.
use macroquad::math::Vec2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymmetryMode {
    None,
    MirrorVertical,
    MirrorHorizontal,
    MirrorBoth,
    Center,
}
impl SymmetryMode {
    pub fn label(&self) -> &'static str {
        match self {
            SymmetryMode::None => "None",
            SymmetryMode::MirrorVertical => "--",
            SymmetryMode::MirrorHorizontal => "|",
            SymmetryMode::MirrorBoth => "-|-",
            SymmetryMode::Center => ".",
        }
    }
    pub const ALL: [SymmetryMode; 5] = [
        SymmetryMode::None,
        SymmetryMode::MirrorVertical,
        SymmetryMode::MirrorHorizontal,
        SymmetryMode::MirrorBoth,
        SymmetryMode::Center,
    ];
    /// Calculates symmetric positions.
    /// `pos`: original world position.
    /// `map_w`, `map_h`: map dimensions.
    /// Diagonal/AntiDiagonal modes perform point reflection relative to map center/axes.
    pub fn symmetric_positions(&self, pos: Vec2, map_w: f32, map_h: f32) -> Vec<Vec2> {
        let x = pos.x;
        let y = pos.y;
        let mut positions = vec![pos];
        match self {
            SymmetryMode::None => {}
            SymmetryMode::MirrorVertical => {
                positions.push(Vec2::new(x, map_h - 1.0 - y));
            }
            SymmetryMode::MirrorHorizontal => {
                positions.push(Vec2::new(map_w - 1.0 - x, y));
            }
            SymmetryMode::MirrorBoth => {
                positions.push(Vec2::new(map_w - 1.0 - x, y));
                positions.push(Vec2::new(x, map_h - 1.0 - y));
                positions.push(Vec2::new(map_w - 1.0 - x, map_h - 1.0 - y));
            }
            SymmetryMode::Center => {
                positions.push(Vec2::new(map_w - 1.0 - x, map_h - 1.0 - y));
            }
        }
        // Remove duplicates (e.g. if original pos is on symmetry line).
        positions.dedup_by(|a, b| (a.x - b.x).abs() < 0.01 && (a.y - b.y).abs() < 0.01);
        positions
    }
}
