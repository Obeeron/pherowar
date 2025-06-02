use macroquad::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolType {
    Food,
    Wall,
    Colony,
}

impl ToolType {
    pub fn all() -> &'static [ToolType] {
        &[ToolType::Food, ToolType::Wall, ToolType::Colony]
    }

    pub fn label(&self) -> &'static str {
        match self {
            ToolType::Food => "Food",
            ToolType::Wall => "Wall",
            ToolType::Colony => "Colony",
        }
    }

    pub fn is_sizeable(&self) -> bool {
        match self {
            ToolType::Food => true,
            ToolType::Wall => true,
            ToolType::Colony => false,
        }
    }
}
