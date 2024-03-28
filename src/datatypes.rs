#[derive(Debug)]
pub struct Vertex {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug)]
pub struct Node {
    pub vertex: Vertex,
    pub ux: f64,
    pub uy: f64,
    pub fx: f64,
    pub fy: f64,
}

#[derive(Debug)]
pub struct Element {
    pub nodes: [usize; 3],
}
