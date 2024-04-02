/*

--- Magnetite ---

Magnetite is a 2D finite-element solver for linear-elastic mechanical
problems.

Kyle Tennison
March 29, 2024

*/

use clap::{ArgAction, Parser};
use error::MagnetiteError;
mod datatypes;
mod error;
mod mesher;
mod post_processor;
mod solver;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(
        short,
        long,
        index = 1,
        value_name = "FILE",
        help = "Input Json with boundary conditions"
    )]
    input_file: String,

    #[arg(short, long, index=2, required=true, value_name="FILE", num_args=0.., help="Geometry SVG or CSVs")]
    geometry_files: Vec<String>,

    #[arg(short, long, default_value = "coolwarm", help = "cmap for python plot")]
    cmap: String,

    #[arg(short, long, help = "skip python plot")]
    skip: bool,
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

    if !args.skip {
        post_processor::pyplot(nodes_output, elements_output, &args.cmap)?;
    }

    Ok(())
}
