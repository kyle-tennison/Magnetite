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

#[derive(Debug)]
pub struct ModelMetadata {
    pub youngs_modulus: f64,
    pub poisson_ratio: f64,
    pub part_thickness: f64,
    pub characteristic_length: f32,
    pub characteristic_length_variance: f32,
}

#[derive(Debug)]
pub struct BoundaryRegion {
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
}

#[derive(Debug)]
pub struct BoundaryTarget {
    pub ux: Option<f64>,
    pub uy: Option<f64>,
    pub fx: Option<f64>,
    pub fy: Option<f64>,
}

#[derive(Debug)]
pub struct BoundaryRule {
    pub name: String,
    pub region: BoundaryRegion,
    pub target: BoundaryTarget,
}
