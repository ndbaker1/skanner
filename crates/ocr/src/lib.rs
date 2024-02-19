use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct OCRWord {
    pub x1: u32,
    pub y1: u32,
    pub x2: u32,
    pub y2: u32,
    pub confidence: u32,
    pub text: String,
}
