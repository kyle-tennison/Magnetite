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
            vertex: Vertex { x: 0.0, y: 0.0 },
            ux: 0.0,
            uy: 0.0,
            fx: 0.0,
            fy: 0.0,
        },
        Node {
            vertex: Vertex { x: 5.0, y: 0.0 },
            ux: 0.0,
            uy: 0.0,
            fx: 0.0,
            fy: 0.0,
        },
        Node {
            vertex: Vertex { x: 5.0, y: 5.0 },
            ux: 0.0,
            uy: 0.0,
            fx: 0.0,
            fy: 0.0,
        },
    ];

    let elements = vec![Element { nodes: [0, 1, 2] }];

    solver::run(nodes, elements, 69e9, 1.0, 0.25)
}
