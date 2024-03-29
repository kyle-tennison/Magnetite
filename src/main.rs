use datatypes::{Element, Node, Vertex};

mod datatypes;
mod error;
mod mesher;
mod post_processor;
mod solver;

fn main() {
    let (mut nodes, mut elements) = mesher::run("vertices.csv", 15.0, 5.0).unwrap();

    return;
    solver::run(&mut nodes, &mut elements, 30e6, 0.5, 0.25).unwrap();

    let current_dir = std::env::current_dir().unwrap();
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

    let nodes_output = "nodes.csv";
    let elements_output = "elements.csv";
    post_processor::csv_output(&elements, &nodes, nodes_output, elements_output).unwrap();
    post_processor::pyplot(nodes_output, elements_output, plotter_path.as_str()).unwrap();

    std::fs::remove_file(nodes_output).unwrap();
    std::fs::remove_file(elements_output).unwrap();
}
