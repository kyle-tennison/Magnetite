use std::io::{Read, Write};

use json::JsonValue;

use crate::{
    datatypes::{
        BoundaryRegion, BoundaryRule, BoundaryTarget, Element, ModelMetadata, Node, Vertex,
    },
    error::MagnetiteError,
};

enum MeshParseState {
    Nodes,
    Elements,
    Entities,
    Limbo,
}

/// Parses a .svg file into a list of Vertexes
///
/// # Arguments
///
/// * `svg_file` - The path to the input svg file
///
/// # Returns
///
/// An ordered vector of Vertex instances
fn parse_svg(svg_file: &str) -> Result<Vec<Vertex>, MagnetiteError> {
    let contents = match std::fs::read_to_string(svg_file) {
        Ok(file) => file,
        Err(_err) => {
            return Err(MagnetiteError::Input(format!(
                "Unable to open svg file {}",
                svg_file
            )));
        }
    };

    let doc = roxmltree::Document::parse(&contents).unwrap();

    let polyline = match doc
        .descendants()
        .find(|n| n.tag_name().name() == "polyline")
    {
        Some(p) => p,
        None => {
            return Err(MagnetiteError::Input(
                "Error in svg file. No polyline element.".to_string(),
            ));
        }
    };

    let points_raw = match polyline.attribute("points") {
        Some(p) => p,
        None => {
            return Err(MagnetiteError::Input(
                "Error in svg file. No points in polyline element.".to_string(),
            ))
        }
    }
    .split(" ");

    let mut points_nopair: Vec<f64> = Vec::new();

    for point_str in points_raw {
        let point: f64 = point_str.parse().expect("Non-float value in svg points");
        points_nopair.push(point);
    }

    let mut points: Vec<Vertex> = Vec::new();
    let mut i: usize = 0;
    while i < points_nopair.len() {
        let x = points_nopair[i];
        let y = points_nopair[i + 1];

        points.push(Vertex { x, y });

        i += 2;
    }

    println!(
        "info: successfully loaded {} vertices from svg",
        points.len()
    );

    Ok(points)
}

/// Parses a CSV file into a list of vertices
///
/// # Arguments
///
/// * `csv_file` - The path to the input csv file
///
/// # Returns
///
/// An ordered vector of Vertex objects
fn parse_csv(csv_file: &str) -> Result<Vec<Vertex>, MagnetiteError> {
    let contents = match std::fs::read_to_string(csv_file) {
        Ok(c) => c,
        Err(_err) => {
            return Err(MagnetiteError::Input(format!(
                "Unable to open csv file {}",
                csv_file
            )))
        }
    };

    let mut headers: Vec<&str> = Vec::new();
    let mut x_index: usize = 0;
    let mut y_index: usize = 0;
    let mut vertices: Vec<Vertex> = Vec::new();

    for line in contents.split("\n") {
        if line.is_empty() {
            continue;
        }

        if headers.len() == 0 {
            headers = line.split(",").map(|x| x.trim()).collect();

            if !headers.contains(&"x") || !headers.contains(&"y") {
                return Err(MagnetiteError::Input(
                    "Error in csv file: Missing x and/or y field".to_string(),
                ));
            }

            x_index = headers.iter().position(|f| f == &"x").unwrap();
            y_index = headers.iter().position(|f| f == &"y").unwrap();
        } else {
            let line_contents: Vec<f64> = line
                .split(",")
                .map(|x| x.trim().parse().expect("Non-float value in csv points"))
                .collect();

            let x = line_contents[x_index];
            let y = line_contents[y_index];

            vertices.push(Vertex { x, y });
        }
    }

    Ok(vertices)
}

/// Builds a .geo file with from a list of vertices
///
/// # Arguments
///
/// * `vertices` - The vector of vertices to parse into a geometry
/// * `output_file` - The output .geo file
fn build_geo(
    vertices: &Vec<Vertex>,
    output_file: &str,
    characteristic_length: f32,
    characteristic_length_variance: f32,
) -> Result<(), MagnetiteError> {
    let mut geo_file = std::fs::File::create(output_file).expect("Failed to create .geo file");

    // Define points
    geo_file.write("// Define points\n".as_bytes()).unwrap();
    for (i, vertex) in vertices.iter().enumerate() {
        geo_file
            .write(format!("Point({}) = {{ {}, {}, 0, 1.0 }};\n", i, vertex.x, vertex.y).as_bytes())
            .unwrap();
    }

    // Connect points
    geo_file.write("\n//Connect points\n".as_bytes()).unwrap();
    for i in 1..vertices.len() {
        geo_file
            .write(format!("Line({}) = {{ {}, {} }};\n", i - 1, i - 1, i).as_bytes())
            .unwrap();
    }
    geo_file
        .write(
            format!(
                "Line({}) = {{ {}, {} }};\n",
                vertices.len() - 1,
                vertices.len() - 1,
                0
            )
            .as_bytes(),
        )
        .unwrap();

    // Define outermost loop
    geo_file
        .write("\n//Register outer loop\n".as_bytes())
        .unwrap();
    geo_file.write("Line Loop(1) = {".as_bytes()).unwrap();
    for i in 0..vertices.len() {
        geo_file
            .write(
                format!(
                    "{} {}",
                    ({
                        if i != 0 {
                            ","
                        } else {
                            ""
                        }
                    }),
                    i
                )
                .as_bytes(),
            )
            .unwrap();
    }
    geo_file.write(" };\n".as_bytes()).unwrap();
    geo_file
        .write("Plane Surface(1) = {1};\n".as_bytes())
        .unwrap();

    // Define meshing settings

    geo_file
        .write(
            format!(
                "\n// Define Mesh Settings\n\
                Mesh.ElementOrder = 1;\n\
                Mesh.Algorithm  = 1;\n\
                Mesh.CharacteristicLengthMax = {cl_max};\n\
                Mesh.CharacteristicLengthMin = {cl_min};\n\n\
                Mesh 2;\n\
                ",
                cl_max = characteristic_length + characteristic_length_variance,
                cl_min = characteristic_length - characteristic_length_variance,
            )
            .as_bytes(),
        )
        .unwrap();

    Ok(())
}

/// Runs Gmsh to create a mesh from a list of vertices
///
/// # Arguments
/// * `vertices` - A vector of vertex objects
/// * `output` - The output filepath of the .msh file
/// * `characteristic_length` - Characteristic length of the mesh
/// * `characteristic_length_variance` - Characteristic length variance of the mesh
fn compute_mesh(
    vertices: &Vec<Vertex>,
    output: &str,
    characteristic_length: f32,
    characteristic_length_variance: f32,
) -> Result<(), MagnetiteError> {
    let geo_filepath = "geom.geo";

    println!("info: building .geo for Gmsh with CL={:.3} and CV={:.3}", characteristic_length, characteristic_length_variance);
    build_geo(
        vertices,
        geo_filepath,
        characteristic_length,
        characteristic_length_variance,
    )?;

    println!("info: running gmsh...");
    let _output = match std::process::Command::new("gmsh")
        .arg(geo_filepath)
        .arg("-2")
        .arg("-o")
        .arg(output)
        .output()
    {
        Ok(out) => out,
        Err(err) => {
            return Err(MagnetiteError::Mesher(
                format!("Gmsh failed: {err}").to_string(),
            ));
        }
    };

    std::fs::remove_file(geo_filepath).expect("Failed to delete .geo file");

    Ok(())
}

fn parse_mesh(mesh_file: &str) -> Result<(Vec<Node>, Vec<Element>), MagnetiteError> {
    let mut elements: Vec<Element> = Vec::new();

    let mut mesh_fs = match std::fs::File::open(mesh_file) {
        Ok(f) => f,
        Err(err) => {
            return Err(MagnetiteError::Mesher(format!(
                "Unable to open auto-generated mesh file: {err}"
            )))
        }
    };

    let mut mesh_contents: String = String::new();
    mesh_fs
        .read_to_string(&mut mesh_contents)
        .expect("Failed to read mesh contents into String");

    let mut parser_state = MeshParseState::Limbo;
    let mut parsed_section_metadata = false;
    let mut lines = mesh_contents.split("\n");

    let mut nodes_unordered: Vec<Node> = Vec::new();
    let mut node_indexes: Vec<usize> = Vec::new();

    while let Some(line) = lines.next() {
        if line.is_empty() {
            continue;
        }

        if line.starts_with("$End") {
            parser_state = MeshParseState::Limbo;
        }

        match parser_state {
            MeshParseState::Limbo => {
                parsed_section_metadata = false;

                if line.starts_with("$Entities") {
                    parser_state = MeshParseState::Entities;
                } else if line.starts_with("$Node") {
                    parser_state = MeshParseState::Nodes;
                } else if line.starts_with("$Elements") {
                    parser_state = MeshParseState::Elements;
                }
                continue;
            }
            MeshParseState::Nodes => {
                if !parsed_section_metadata {
                    parsed_section_metadata = true;
                    continue;
                }

                let node_data: Vec<usize> = line
                    .split(" ")
                    .map(|i| i.parse().expect("Unexpected non-int in mesh data"))
                    .collect();

                let num_nodes_local = node_data[3];

                let mut node_tags: Vec<usize> = Vec::with_capacity(num_nodes_local);

                for _ in 0..num_nodes_local {
                    let tag: usize = lines
                        .next()
                        .unwrap()
                        .parse()
                        .expect("found non-int node tag");
                    node_tags.push(tag);
                }

                for i in 0..num_nodes_local {
                    let node_coords: Vec<f64> = lines
                        .next()
                        .unwrap()
                        .split(" ")
                        .map(|c| c.parse().expect("Non-float coordinate in mesh"))
                        .collect();

                    let node = Node {
                        vertex: Vertex {
                            x: node_coords[0],
                            y: node_coords[1],
                        },
                        ux: None,
                        uy: None,
                        fx: Some(0.0),
                        fy: Some(0.0),
                    };

                    nodes_unordered.push(node);
                    node_indexes.push(node_tags[i] - 1);
                }
            }
            MeshParseState::Elements => {
                if !parsed_section_metadata {
                    parsed_section_metadata = true;
                    continue;
                }

                let element_data: Vec<usize> = line
                    .split(" ")
                    .map(|i| {
                        i.parse()
                            .expect(format!("Unexpected non-int in mesh data {}", i).as_str())
                    })
                    .collect();

                let entity_dim = element_data[0];
                let num_elements = element_data[3];

                for _ in 0..num_elements {
                    let metadata: Vec<usize> = lines
                        .next()
                        .unwrap()
                        .trim()
                        .split(" ")
                        .map(|i| {
                            i.parse()
                                .expect(format!("Unexpected non-int in mesh data {}", i).as_str())
                        })
                        .collect();

                    if entity_dim != 2 {
                        continue;
                    }

                    let n0 = metadata[1] - 1;
                    let n1 = metadata[2] - 1;
                    let n2 = metadata[3] - 1;

                    elements.push(Element {
                        nodes: [n0, n1, n2],
                        stress: None,
                    })
                }
            }
            MeshParseState::Entities => continue,
        }
    }

    // Order nodes
    let mut nodes: Vec<Node> =
        Vec::with_capacity(std::mem::size_of::<Node>() * nodes_unordered.len());

    // we will be over writing all of these null values
    unsafe {
        nodes.set_len(nodes_unordered.len());
    }

    for (idx, node) in std::iter::zip(node_indexes, nodes_unordered) {
        nodes[idx] = node;
    }

    println!(
        "info: loaded {} nodes and {} elements",
        nodes.len(),
        elements.len()
    );

    std::fs::remove_file(mesh_file).expect("Failed to delete .msh file");

    Ok((nodes, elements))
}

fn load_input_file(input_file: &str) -> Result<JsonValue, MagnetiteError> {
    let file_string = match std::fs::read_to_string(input_file) {
        Ok(f) => f,
        Err(_err) => {
            return Err(MagnetiteError::Input(format!(
                "Unable to open input file {}",
                input_file
            )))
        }
    };

    let input_file_json = match json::parse(&file_string) {
        Ok(f) => f,
        Err(err) => {
            return Err(MagnetiteError::Input(format!(
                "Error in input file json: {err}"
            )))
        }
    };

    if !input_file_json.has_key("metadata") {
        return Err(MagnetiteError::Input(
            "Input json missing metadata filed".to_string(),
        ));
    }
    if !input_file_json.has_key("boundary_conditions") {
        return Err(MagnetiteError::Input(
            "Input json missing boundary_conditions filed".to_string(),
        ));
    }
    if !input_file_json["metadata"].has_key("part_thickness") {
        return Err(MagnetiteError::Input(
            "Input json missing part_thickness filed".to_string(),
        ));
    }
    if !input_file_json["metadata"].has_key("material_elasticity") {
        return Err(MagnetiteError::Input(
            "Input json missing material_elasticity filed".to_string(),
        ));
    }
    if !input_file_json["metadata"].has_key("poisson_ratio") {
        return Err(MagnetiteError::Input(
            "Input json missing poisson_ratio filed".to_string(),
        ));
    }

    Ok(input_file_json)
}

fn parse_input_metadata(input_json: &JsonValue) -> ModelMetadata {
    // todo!("Make this raise a magnetite error:");
    ModelMetadata {
        youngs_modulus: input_json["metadata"]["material_elasticity"]
            .as_f64()
            .expect("error in material_elasticity field"),
        part_thickness: input_json["metadata"]["part_thickness"]
            .as_f64()
            .expect("error in part_thickness field"),
        poisson_ratio: input_json["metadata"]["poisson_ratio"]
            .as_f64()
            .expect("error in poisson_ratio field"),
        characteristic_length: input_json["metadata"]["characteristic_length"]
            .as_f32()
            .expect("error in characteristic_length field"),
        characteristic_length_variance: input_json["metadata"]["characteristic_length_variance"]
            .as_f32()
            .expect("error in characteristic_length_variance field"),
    }
}

fn apply_boundary_conditions(
    input_json: &JsonValue,
    nodes: &mut Vec<Node>,
) -> Result<(), MagnetiteError> {
    let mut rules: Vec<BoundaryRule> = Vec::new();

    // Load rules from json
    for (name, rule_json) in input_json["boundary_conditions"].entries() {
        if !rule_json.has_key("region") {
            return Err(MagnetiteError::Input(format!(
                "Boundary rule {name} is missing region field"
            )));
        }
        if !rule_json.has_key("targets") {
            return Err(MagnetiteError::Input(format!(
                "Boundary rule {name} is missing target field"
            )));
        }

        // Register region
        let mut boundary_region = BoundaryRegion {
            x_min: f64::MIN,
            x_max: f64::MAX,
            y_min: f64::MIN,
            y_max: f64::MAX,
        };
        if rule_json["region"].has_key("x_target_min") {
            boundary_region.x_min = rule_json["region"]["x_target_min"]
                .as_f64()
                .expect(format!("Bad value for x_target_min in {name}").as_str())
        }
        if rule_json["region"].has_key("x_target_max") {
            boundary_region.x_max = rule_json["region"]["x_target_max"]
                .as_f64()
                .expect(format!("Bad value for x_target_max in {name}").as_str())
        }
        if rule_json["region"].has_key("y_target_min") {
            boundary_region.x_min = rule_json["region"]["y_target_min"]
                .as_f64()
                .expect(format!("Bad value for y_target_min in {name}").as_str())
        }
        if rule_json["region"].has_key("y_target_max") {
            boundary_region.x_max = rule_json["region"]["y_target_max"]
                .as_f64()
                .expect(format!("Bad value for y_target_max in {name}").as_str())
        }

        // Register target
        let boundary_target = BoundaryTarget {
            ux: rule_json["targets"]["ux"].as_f64(),
            uy: rule_json["targets"]["uy"].as_f64(),
            fx: rule_json["targets"]["fx"].as_f64(),
            fy: rule_json["targets"]["fy"].as_f64(),
        };

        rules.push(BoundaryRule {
            name: name.to_string(),
            region: boundary_region,
            target: boundary_target,
        })
    }
    println!(
        "info: loaded {} boundary rules from input file",
        &rules.len()
    );

    for node in nodes {
        for rule in &rules {
            let candidate = node.vertex.x > rule.region.x_min
                && node.vertex.x < rule.region.x_max
                && node.vertex.y > rule.region.y_min
                && node.vertex.y < rule.region.y_max;

            if candidate {
                node.ux = rule.target.ux;
                node.uy = rule.target.uy;
                node.fx = rule.target.fx;
                node.fy = rule.target.fy;
            }
        }
    }

    Ok(())
}

/// Runs the mesher
///
/// # Arguments
/// * `geometry_file` - The geometry input file--either csv or svg
/// * `input_file` - The input file that contains boundary conditions
/// * `characteristic_length` - Characteristic length of the mesh
/// * `characteristic_length_variance` - Characteristic length variance of the mesh
pub fn run(
    geometry_file: &str,
    input_file: &str,
) -> Result<(Vec<Node>, Vec<Element>, ModelMetadata), MagnetiteError> {
    let input_file_json = load_input_file(input_file)?;
    let model_metadata = parse_input_metadata(&input_file_json);

    let vertices: Vec<Vertex>;

    if geometry_file.ends_with(".svg") {
        vertices = parse_svg(geometry_file)?;
    } else if geometry_file.ends_with(".csv") {
        vertices = parse_csv(geometry_file)?;
    } else {
        return Err(MagnetiteError::Input(
            format!("Unrecognized geometry filetype {geometry_file}").to_string(),
        ));
    }

    let mesh_filepath = "geom.msh";
    compute_mesh(
        &vertices,
        mesh_filepath,
        model_metadata.characteristic_length,
        model_metadata.characteristic_length_variance,
    )?;

    let (mut nodes, elements) = parse_mesh(mesh_filepath)?;

    apply_boundary_conditions(&input_file_json, &mut nodes)?;

    Ok((nodes, elements, model_metadata))
}
