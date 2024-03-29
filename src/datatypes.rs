#[derive(Debug)]
pub struct Vertex {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug)]
pub struct Node {
    pub vertex: Vertex,
    pub ux: Option<f64>,
    pub uy: Option<f64>,
    pub fx: Option<f64>,
    pub fy: Option<f64>,
}

#[derive(Debug)]
pub struct Element {
    pub nodes: [usize; 3],
    pub stress: Option<f64>,
}
