use crate::datatypes::{Element, Node};
use nalgebra::{matrix, SMatrix};

/// Calculates the area of the element
///
/// # Arguments
/// * `element` - The Element to target
/// * `nodes` - A reference to the vector of nodes
///
/// # Returns
/// The area of the element
fn compute_element_area(element: &Element, nodes: &Vec<Node>) -> f64 {
    let v0 = &nodes[element.nodes[0]].vertex;
    let v1 = &nodes[element.nodes[1]].vertex;
    let v2 = &nodes[element.nodes[2]].vertex;

    0.5 * (v0.x * (v1.y - v2.y) + v1.x * (v2.y - v0.y) + v2.x * (v0.y - v1.y))
}

/// Calculates the strain-displacement matrix of the element
///
/// # Arguments
/// * `element` - The Element to target
/// * `nodes` - A reference to the vector of nodes
/// * `element_area` - The area of the element
///
/// # Returns
/// A 3x6 strain-displacement matrix
fn compute_strain_displacement_matrix(
    element: &Element,
    nodes: &Vec<Node>,
    element_area: f64,
) -> SMatrix<f64, 3, 6> {
    let v0 = &nodes[element.nodes[0]].vertex;
    let v1 = &nodes[element.nodes[1]].vertex;
    let v2 = &nodes[element.nodes[2]].vertex;

    let beta_1 = v1.y - v2.y;
    let beta_2 = v2.y - v0.y;
    let beta_3 = v0.y - v1.y;

    let gamma_1 = v2.x - v1.x;
    let gamma_2 = v0.x - v2.x;
    let gamma_3 = v1.x - v0.x;

    let mut strain_displacement_mat: SMatrix<f64, 3, 6> = matrix![
        beta_1, 0., beta_2, 0., beta_3, 0.;
        0., gamma_1, 0., gamma_2, 0., gamma_3;
        gamma_1, beta_1, gamma_2, beta_2, gamma_3, beta_3;
    ];

    strain_displacement_mat *= element_area;

    strain_displacement_mat
}

/// Calculates the stress-strain matrix
///
/// # Arguments
/// * `poisson_ratio` - The poisson ratio for the model
/// * `youngs_modulus` - The modulus of elasticity of the model
///
/// # Returns
/// A 3x3 stress-strain matrix
fn compute_stress_strain_matrix(poisson_ratio: f64, youngs_modulus: f64) -> SMatrix<f64, 3, 3> {
    let mut strain_stress_mat: SMatrix<f64, 3, 3> = matrix![
        1.0, poisson_ratio, 0.0;
        poisson_ratio, 1.0, 0.0;
        0.0, 0.0, (1.0 - poisson_ratio)/2.0;
    ];

    strain_stress_mat *= youngs_modulus / (1.0 - f64::powi(poisson_ratio, 2));

    strain_stress_mat
}

/// Computes the stiffness matrix for a given element
///
/// # Arguments
/// - `element` - The element to target
/// - `nodes` - A reference to the vector of nodes
/// * `poisson_ratio` - The poisson ratio for the model
/// * `youngs_modulus` - The modulus of elasticity of the model
/// * `part_thickness` - The thickness of the part
///
/// # Returns
/// A 6x6 stiffness matrix for the element
pub fn compute_element_stiffness_matrix(
    element: &Element,
    nodes: &Vec<Node>,
    poisson_ratio: f64,
    youngs_modulus: f64,
    part_thickness: f64,
) -> SMatrix<f64, 6, 6> {
    let element_area = compute_element_area(element, nodes);
    let stress_strain_mat = compute_stress_strain_matrix(poisson_ratio, youngs_modulus);
    let strain_displacement_mat = compute_strain_displacement_matrix(element, nodes, element_area);

    (strain_displacement_mat.transpose() * stress_strain_mat)
        * strain_displacement_mat
        * element_area
        * part_thickness
}

pub fn run(
    nodes: Vec<Node>,
    elements: Vec<Element>,
    youngs_modulus: f64,
    part_thickness: f64,
    poisson_ratio: f64,
) {
    for element in elements {
        println!(
            "{}",
            compute_element_stiffness_matrix(
                &element,
                &nodes,
                poisson_ratio,
                youngs_modulus,
                part_thickness
            )
        )
    }
}
