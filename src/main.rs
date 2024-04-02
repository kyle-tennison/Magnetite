/*

--- Magnetite ---

Magnetite is a 2D finite-element solver for linear-elastic mechanical
problems.

Kyle Tennison
March 29, 2024

*/

use clap::Parser;
use error::MagnetiteError;
mod datatypes;
mod error;
mod mesher;
mod post_processor;
mod solver;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, index = 1, value_name = "FILE")]
    input_file: String,

    #[arg(short, long, index=2, value_name="FILE", num_args=0..)]
    geometry_files: Vec<String>,

    #[arg(short, long, default_value = "coolwarm")]
    cmap: String,
}

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
    let args = Args::parse();

    // Parse input files
    let (mut nodes, mut elements, model_metadata) = mesher::run(
        args.geometry_files.iter().map(|f| f.as_str()).collect(),
        &args.input_file,
    )?;

    // Run simulation
    solver::run(&mut nodes, &mut elements, &model_metadata)?;

    // Output
    let nodes_output = "nodes.csv";
    let elements_output = "elements.csv";
    post_processor::csv_output(&elements, &nodes, nodes_output, elements_output)?;
    post_processor::pyplot(nodes_output, elements_output, &args.cmap)?;

    Ok(())
}
