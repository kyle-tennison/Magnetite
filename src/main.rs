/*

--- Magnetite ---

Magnetite is a 2D finite-element solver for linear-elastic mechanical
problems.

Kyle Tennison
March 29, 2024

*/

use error::MagnetiteError;
use std::env;
mod datatypes;
mod error;
mod mesher;
mod post_processor;
mod solver;

fn main() {
    match entry() {
        Ok(_) => (),
        Err(err) => {
            eprintln!("Received error: {err}");
            std::process::exit(1);
        }
    }
}

/// Entry point to simulator
fn entry() -> Result<(), MagnetiteError> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        println!("usage: magnetite <input_json> <geometry_outer> <geometry_inner ...>");
        std::process::exit(1)
    }

    // Parse input files
    let (mut nodes, mut elements, model_metadata) = mesher::run(
        args[2..].iter().map(|f| f.as_str()).collect(),
        args[1].as_str(),
    )?;

    // Run simulation
    solver::run(&mut nodes, &mut elements, &model_metadata)?;

    // Output
    let nodes_output = "nodes.csv";
    let elements_output = "elements.csv";
    post_processor::csv_output(&elements, &nodes, nodes_output, elements_output)?;
    post_processor::pyplot(nodes_output, elements_output)?;

    Ok(())
}
