use std::{
    io::{BufWriter, Write},
    process::ExitStatus,
};

use crate::{
    datatypes::{Element, Node},
    error::MagnetiteError,
};

/// Writes simulation results to two CSV files
///
/// # Arguments
/// * `elements` - A reference to the vector of post-solve elements
/// * `nodes` - A reference to the vector of post-solve nodes
/// * `nodes_output` - The filename of the output nodes csv
/// * `elements_output` - The filename of the output elements csv
pub fn csv_output(
    elements: &Vec<Element>,
    nodes: &Vec<Node>,
    nodes_output: &str,
    elements_output: &str,
) -> Result<(), MagnetiteError> {
    let mut nodes_file = match std::fs::File::create(nodes_output) {
        Ok(f) => f,
        Err(err) => {
            return Err(MagnetiteError::Solver(format!(
                "Failed to create nodes.csv: {err}"
            )));
        }
    };
    let mut elements_file = match std::fs::File::create(elements_output) {
        Ok(f) => f,
        Err(err) => {
            return Err(MagnetiteError::Solver(format!(
                "Failed to create elements.csv: {err}"
            )));
        }
    };

    // Write nodes
    nodes_file.write("x,y,ux,uy\n".as_bytes()).unwrap();
    for node in nodes {
        nodes_file
            .write(
                format!(
                    "{x},{y},{ux},{uy}\n",
                    x = node.vertex.x,
                    y = node.vertex.y,
                    ux = node.ux.unwrap(),
                    uy = node.uy.unwrap(),
                )
                .as_bytes(),
            )
            .unwrap();
    }

    // Write vertices
    elements_file
        .write(format!("n0,n1,n2,stress\n").as_bytes())
        .unwrap();
    for element in elements {
        elements_file
            .write(
                format!(
                    "{n0},{n1},{n2},{stress}\n",
                    n0 = element.nodes[0],
                    n1 = element.nodes[1],
                    n2 = element.nodes[2],
                    stress = element.stress.unwrap()
                )
                .as_bytes(),
            )
            .unwrap();
    }

    println!(
        "info: wrote output to {} and {}",
        nodes_output, elements_output
    );

    Ok(())
}

/// Calls the python plotter to plot results
///
/// # Arguments
/// * `nodes_csv` - The filepath to the nodes csv output
/// * `elements_csv` - The filepath to the elements csv output
pub fn pyplot(nodes_csv: &str, elements_csv: &str, cmap: &str) -> Result<(), MagnetiteError> {
    // resolve plotter path
    let current_dir = std::env::current_exe().unwrap();
    let repo_dir = current_dir
        .ancestors()
        .into_iter()
        .find(|p| p.ends_with("Magnetite"))
        .expect("Unable to find root repo directory");
    let plotter_path = repo_dir
        .join("scripts/plot.py")
        .canonicalize()
        .expect("Unable to find plotter script")
        .into_os_string()
        .into_string()
        .unwrap();

    println!("info: plotting in python...");
    let res = std::process::Command::new("python")
        .arg(plotter_path)
        .arg(nodes_csv)
        .arg(elements_csv)
        .arg(cmap)
        .output()
        .unwrap();

    if !res.status.success() {
        return Err(MagnetiteError::PostProcessor(format!(
            "error: python plotter raised error:\n\n{}",
            String::from_utf8_lossy(res.stderr.iter().as_slice())
        )));
    }

    Ok(())
}
