use crate::{
    datatypes::{Element, ModelMetadata, Node},
    error::MagnetiteError,
};
use indicatif::ProgressBar;
use nalgebra::{matrix, DMatrix, DVector, SMatrix};

use argmin::{
    core::{
        observers::{Observe, ObserverMode},
        ArgminFloat, Error, Executor, Operator, State, KV,
    },
    solver::conjugategradient::ConjugateGradient,
};

pub const DOF: usize = 2;
pub const MAX_CG_ITER: u64 = 1e7 as u64;
pub const TARGET_CG_COST: f64 = 1e-4 as f64;

/// Runs multiplication for Conjugate Gradient Solver
struct ConjugateGradientOperator<'a> {
    a: &'a DMatrix<f64>, // TODO: Use a sparse matrix to speed up multiplication times
}

impl<'a> Operator for ConjugateGradientOperator<'a> {
    type Param = Vec<f64>;
    type Output = Vec<f64>;

    fn apply(&self, x: &Self::Param) -> Result<Self::Output, argmin::core::Error> {
        Ok((self.a * DVector::from_vec(x.to_vec()))
            .data
            .as_vec()
            .clone())
    }
}

/// Observer bar for argmin solver
struct ConjugateGradientObserverBar {
    bar: ProgressBar,
    final_mag: f64,
}

impl ConjugateGradientObserverBar {
    fn new() -> ConjugateGradientObserverBar {
        ConjugateGradientObserverBar {
            bar: ProgressBar::new(1000),
            final_mag: TARGET_CG_COST.log10().floor(),
        }
    }

    fn argmin_float_to_f64<F: ArgminFloat>(&self, value: F) -> Option<f64> {
        // TODO: There absolutely should be a way to extract the value
        // from a ArgminFloat instance that doesn't need this
        match format!("{:?}", value).parse() {
            Ok(n) => Some(n),
            Err(_) => None,
        }
    }
}

impl<I> Observe<I> for ConjugateGradientObserverBar
where
    I: State,
{
    fn observe_init(&mut self, _name: &str, _state: &I, _kv: &KV) -> Result<(), Error> {
        Ok(())
    }

    fn observe_iter(&mut self, state: &I, _kv: &KV) -> Result<(), Error> {
        let cost = match self.argmin_float_to_f64(state.get_cost()) {
            Some(c) => c,
            None => return Ok(()), // skip if we can't parse
        };
        let cost_mag = cost.log10().floor();
        let progress = (1000. / f64::sqrt(cost_mag - self.final_mag)) as u64;
        self.bar.set_position(progress);

        Ok(())
    }

    fn observe_final(&mut self, _state: &I) -> Result<(), Error> {
        self.bar.finish();
        Ok(())
    }
}

/// Solves a system of equations using the conjugate gradient method.
///
/// This function returns an approximation for x in `Ax=b`
///
/// # Arguments
/// * `a` - A square positive definite matrix
/// * `b` - A vector of the solutions to the system
///
/// # Returns
/// A DVector that represents `x` from the system
fn run_conjugate_gradient(
    a: &DMatrix<f64>,
    b: &DVector<f64>,
) -> Result<DVector<f64>, MagnetiteError> {
    let b_flat: Vec<f64> = b.iter().map(|f| *f).collect();
    let solver: ConjugateGradient<_, f64> = ConjugateGradient::new(b_flat);
    let initial_guess: Vec<f64> = vec![0.0; b.nrows()];

    let operator = ConjugateGradientOperator { a };
    let observer = ConjugateGradientObserverBar::new();

    // Run solver
    let res = match Executor::new(operator, solver)
        .configure(|state| {
            state
                .param(initial_guess)
                .max_iters(MAX_CG_ITER)
                .target_cost(TARGET_CG_COST)
        })
        .add_observer(observer, ObserverMode::NewBest)
        .run()
    {
        Ok(r) => r,
        Err(err) => {
            return Err(MagnetiteError::Solver(format!(
                "Conjugate Gradient error: {err}"
            )))
        }
    };

    let best_param = match &res.state().best_param {
        Some(vec) => DVector::from_vec(vec.clone()),
        None => {
            return Err(MagnetiteError::Solver(
                "Conjugate Gradient could not produce best parameter".to_owned(),
            ))
        }
    };

    Ok(best_param)
}

/// Calculates the area of the element
///
/// # Arguments
/// * `element` - The Element to target
/// * `nodes` - A reference to the vector of nodes
///
/// # Returns
/// The area of the element
pub fn compute_element_area(element: &Element, nodes: &Vec<Node>) -> f64 {
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
pub fn compute_strain_displacement_matrix(
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

    strain_displacement_mat /= 2.0 * element_area;

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
pub fn compute_stress_strain_matrix(poisson_ratio: f64, youngs_modulus: f64) -> SMatrix<f64, 3, 3> {
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
fn compute_element_stiffness_matrix(
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

/// Compiles element stiffness matrices into a total stiffness matrix
///
/// # Arguments
/// * `nodes` - A reference to the vector of nodes
/// * `elements` - A reference to the vector of elements
/// * `element_stiffness_matrices` - A vector of element stiffness matrices
///     that corresponds to the `elements` vector.
///
/// # Returns
/// A dynamically sized matrix
fn build_total_stiffness_matrix(
    nodes: &Vec<Node>,
    elements: &Vec<Element>,
    element_stiffness_matrices: Vec<SMatrix<f64, 6, 6>>,
) -> DMatrix<f64> {
    let mut total_stiffness_matrix: DMatrix<f64> =
        DMatrix::zeros(DOF * nodes.len(), DOF * nodes.len());

    let bar = ProgressBar::new(elements.len() as u64);
    for (i, (stiffness_mat, element)) in
        std::iter::zip(element_stiffness_matrices, elements).enumerate()
    {
        bar.inc(i as u64);

        for (local_row, node_row) in element.nodes.iter().enumerate() {
            for (local_col, node_col) in element.nodes.iter().enumerate() {
                let global_row = node_row * 2;
                let global_col = node_col * 2;
                let local_row = local_row * 2;
                let local_col = local_col * 2;

                // Add RowX ColX
                total_stiffness_matrix[(global_row, global_col)] +=
                    stiffness_mat[(local_row, local_col)];
                // Add RowX ColY
                total_stiffness_matrix[(global_row, global_col + 1)] +=
                    stiffness_mat[(local_row, local_col + 1)];
                // Add RowY ColX
                total_stiffness_matrix[(global_row + 1, global_col)] +=
                    stiffness_mat[(local_row + 1, local_col)];
                // Add RowY ColY
                total_stiffness_matrix[(global_row + 1, global_col + 1)] +=
                    stiffness_mat[(local_row + 1, local_col + 1)];
            }
        }
    }
    bar.finish_with_message(format!("info: successfully build total stiffness matrix\n"));

    total_stiffness_matrix
}

/// Creates nodal forces and nodal displacement column vectors
///
/// # Arguments
/// * `nodes` - The list of nodes
///
/// # Returns
/// The nodal forces and nodal displacements column vectors, in that order
fn build_col_vecs(nodes: &Vec<Node>) -> (Vec<Option<f64>>, Vec<Option<f64>>) {
    let mut nodal_forces: Vec<Option<f64>> =
        Vec::with_capacity(std::mem::size_of::<Option<f64>>() * nodes.len() * DOF);
    let mut nodal_displacements: Vec<Option<f64>> =
        Vec::with_capacity(std::mem::size_of::<Option<f64>>() * nodes.len() * DOF);

    for node in nodes {
        nodal_forces.push(node.fx);
        nodal_forces.push(node.fy);
        nodal_displacements.push(node.ux);
        nodal_displacements.push(node.uy);
    }

    (nodal_forces, nodal_displacements)
}

/// Builds known and unknown matrices. These are used to solve the system
///
/// # Arguments
/// * `nodal_forces` - The nodal forces column vector
/// * `nodal_displacements` - The nodal displacements column vector
/// * `total_stiffness_matrix` - The total stiffness matrix of the model
///
/// # Returns
/// A tuple of the known matrix and the unknown matrix, in that order
fn build_known_unknown_matrices(
    nodal_forces: &Vec<Option<f64>>,
    nodal_displacements: &Vec<Option<f64>>,
    total_stiffness_matrix: &DMatrix<f64>,
) -> (DMatrix<f64>, DMatrix<f64>) {
    let num_known_displacements = nodal_displacements.iter().filter(|x| x.is_some()).count();
    let num_unknown_displacements = nodal_displacements.len() - num_known_displacements;

    let mut known_matrix: DMatrix<f64> =
        DMatrix::zeros(num_unknown_displacements, num_known_displacements);
    let mut unknown_matrix: DMatrix<f64> =
        DMatrix::zeros(num_unknown_displacements, num_unknown_displacements);

    let mut local_row = 0;

    for (row, nodal_force) in nodal_forces.iter().enumerate() {
        if nodal_force.is_none() {
            continue;
        }

        let mut known_idx: usize = 0;
        let mut unknown_idx: usize = 0;

        for (col, nodal_displacement) in nodal_displacements.iter().enumerate() {
            if let Some(nodal_displacement) = nodal_displacement {
                known_matrix[(local_row, known_idx)] =
                    total_stiffness_matrix[(row, col)] * *nodal_displacement;
                known_idx += 1;
            } else {
                unknown_matrix[(local_row, unknown_idx)] = total_stiffness_matrix[(row, col)];
                unknown_idx += 1;
            }
        }

        local_row += 1;
    }

    known_matrix *= -1.0;
    (known_matrix, unknown_matrix)
}

/// Solves for the displacements in the nodes. Loads the results into the node
/// objects
///
/// # Arguments
/// * `nodes` - The vector of nodes
/// * `total_stiffness_matrix` - The total stiffness matrix of the model
fn solve(
    nodes: &mut Vec<Node>,
    total_stiffness_matrix: &DMatrix<f64>,
) -> Result<(), MagnetiteError> {
    println!("info: setting up system...");

    // Assemble column Matrixes
    let (mut nodal_forces, mut nodal_displacements) = build_col_vecs(nodes);

    // Setup equation for unknown displacements
    let (known_matrix, unknown_matrix) =
        build_known_unknown_matrices(&nodal_forces, &nodal_displacements, total_stiffness_matrix);

    let mut known_matrix_summed: DVector<f64> = known_matrix.column_sum();
    let known_forces: Vec<&Option<f64>> = nodal_forces.iter().filter(|x| x.is_some()).collect();

    for (i, k) in known_matrix_summed.iter_mut().enumerate() {
        *k += known_forces[i].unwrap();
    }

    // Solve for nodal displacements
    let start = std::time::Instant::now();

    println!("info: solving...");
    let displacement_solution = run_conjugate_gradient(&unknown_matrix, &known_matrix_summed)?;

    let elapsed = (std::time::Instant::now() - start).as_secs_f32();
    println!("info: solved system in {:.3} seconds", elapsed);

    // Load displacement solution into nodal_displacement vector
    let mut solution_cursor = 0;
    for u in nodal_displacements.iter_mut() {
        if u.is_none() {
            *u = Some(displacement_solution[(solution_cursor, 0)]);
            solution_cursor += 1;
        }
    }
    let nodal_displacements: Vec<f64> = nodal_displacements
        .iter()
        .map(|u| u.expect("Unknown displacement after solve"))
        .collect();

    // Solve for forces
    for (i, f) in nodal_forces.iter_mut().enumerate() {
        if f.is_some() {
            continue;
        }

        let mut solved_force: f64 = 0.0;

        for col in 0..nodal_displacements.len() {
            solved_force += total_stiffness_matrix[(i, col)] * nodal_displacements[col]
        }

        *f = Some(solved_force);
    }
    let nodal_forces: Vec<f64> = nodal_forces
        .iter()
        .map(|f| f.expect("Unknown force after solve"))
        .collect();

    // Load results into nodes
    for (i, node) in nodes.iter_mut().enumerate() {
        node.ux = Some(nodal_displacements[2 * i]);
        node.uy = Some(nodal_displacements[2 * i + 1]);

        node.fx = Some(nodal_forces[2 * i]);
        node.fy = Some(nodal_forces[2 * i + 1]);
    }

    println!("info: solve complete");

    Ok(())
}

/// Calculates the stress in an element
///
/// # Arguments
/// * `elements` - A mutable reference to the vector of elements
/// * `nodes` - A mutable reference to the vector of nodes
/// * `poisson_ratio` - The model's poisson ratio
/// * `youngs_modulus` - The model's material elasticity
fn compute_stress(
    elements: &mut Vec<Element>,
    nodes: &mut Vec<Node>,
    poisson_ratio: f64,
    youngs_modulus: f64,
) {
    for element in elements {
        let element_nodes = Vec::from(element.nodes.map(|i| &nodes[i]));

        let nodal_displacements: [f64; 6] = [
            element_nodes[0].ux.unwrap(),
            element_nodes[0].uy.unwrap(),
            element_nodes[1].ux.unwrap(),
            element_nodes[1].uy.unwrap(),
            element_nodes[2].ux.unwrap(),
            element_nodes[2].uy.unwrap(),
        ];

        let displacement_mat: SMatrix<f64, { DOF * 3 }, 1> = SMatrix::from(nodal_displacements);

        let stress = compute_stress_strain_matrix(poisson_ratio, youngs_modulus)
            * compute_strain_displacement_matrix(
                element,
                &nodes,
                compute_element_area(element, &nodes),
            )
            * displacement_mat;

        element.stress = Some(f64::sqrt(f64::powi(stress[0], 2) + f64::powi(stress[1], 2)));
    }
}

/// Runs the solver. Updates values on nodes and elements vectors
///
/// # Arguments
/// * `elements` - A mutable reference to the vector of elements
/// * `nodes` - A mutable reference to the vector of nodes
/// * `model_metadata` - The model metadata
pub fn run(
    nodes: &mut Vec<Node>,
    elements: &mut Vec<Element>,
    model_metadata: &ModelMetadata,
) -> Result<(), MagnetiteError> {
    // Build element stiffness matrix for each element
    let mut element_stiffness_matrices: Vec<SMatrix<f64, 6, 6>> = Vec::new();

    println!("info: building element stiffness matrices...");
    let bar = ProgressBar::new(elements.len() as u64);
    for (i, element) in elements.iter().enumerate() {
        bar.inc(i as u64);

        element_stiffness_matrices.push(compute_element_stiffness_matrix(
            &element,
            &nodes,
            model_metadata.poisson_ratio,
            model_metadata.youngs_modulus,
            model_metadata.part_thickness,
        ));
    }
    bar.finish_with_message(format!(
        "info: successfully built {} stiffness matrices\n",
        elements.len()
    ));

    // Compile matrices into total stiffness matrix
    println!("info: building total stiffness matrix...");
    let total_stiffness_matrix =
        build_total_stiffness_matrix(&nodes, &elements, element_stiffness_matrices);

    // Solve system
    solve(nodes, &total_stiffness_matrix)?;

    // Solve for stress
    compute_stress(
        elements,
        nodes,
        model_metadata.poisson_ratio,
        model_metadata.youngs_modulus,
    );

    Ok(())
}
