use datatypes::{Element, Node, Vertex};

mod datatypes;
mod error;
mod mesher;
mod solver;
extern crate nalgebra as na;

fn main() {
    // mesher::run("vertices.csv", 15.0, 5.0).unwrap();

    // nodes = [
    // Node(0,0,None,None,None,None, 0),
    // Node(5,0,None,None,None,None, 1),
    // Node(5,5,None,None,None,None, 2),
    // ]

    // element = Element(*nodes)
    // element.poisson_ratio = 0.25
    // element.material_elasticity = int(69e9)
    // element.part_thickness = 1

    let nodes = vec![
        Node {
            vertex: Vertex { x: 3.0, y: 0.0 },
            ux: None,
            uy: Some(0.0),
            fx: Some(0.0),
            fy: None,
        },
        Node {
            vertex: Vertex { x: 3.0, y: 2.0 },
            ux: None,
            uy: None,
            fx: Some(0.0),
            fy: Some(-1000.0),
        },
        Node {
            vertex: Vertex { x: 0.0, y: 2.0 },
            ux: Some(0.0),
            uy: Some(0.0),
            fx: None,
            fy: None,
        },
        Node {
            vertex: Vertex { x: 0.0, y: 0.0 },
            ux: Some(0.0),
            uy: Some(0.0),
            fx: None,
            fy: None,
        },
    ];

    let elements = vec![Element { nodes: [0, 1, 3] }, Element { nodes: [2, 3, 1] }];

    solver::run(nodes, elements, 30e6, 0.5, 0.25)
}
