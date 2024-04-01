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
/// * `svg_file` - The path to the input svg file
///
/// # Returns
/// An ordered vector of Vertex instances
fn parse_svg(svg_file: &str, min_element_length: f32) -> Result<Vec<Vec<Vertex>>, MagnetiteError> {
    let contents = match std::fs::read_to_string(svg_file) {
        Ok(file) => file,
        Err(_err) => {
            return Err(MagnetiteError::Input(format!(
                "Unable to open svg file {}",
                svg_file
            )));
        }
    };

    let mut skipped_vertices: usize = 0; // count number of skips

    // Parse polylines and polygons from svg xml
    let doc = roxmltree::Document::parse(&contents).unwrap();
    let polylines: Vec<roxmltree::Node> = doc
        .descendants()
        .into_iter()
        .filter(|n| n.tag_name().name() == "polyline" || n.tag_name().name() == "polygon")
        .collect();

    let mut vertex_containers: Vec<Vec<Vertex>> = Vec::new();
    vertex_containers.push(Vec::new()); // placeholder for outer

    for polyline in polylines {
        // Read points from points attribute
        let points_raw = match polyline.attribute("points") {
            Some(p) => p,
            None => {
                return Err(MagnetiteError::Input(format!(
                    "Error in svg file. No points in polyline element {:?}",
                    polyline.id()
                )))
            }
        }
        .split(" ");

        // Parse points into vertices
        let mut points: Vec<Vertex> = Vec::new();
        let mut points_nopair: Vec<f64> = Vec::new();
        for point_str in points_raw {
            let point: f64 = point_str.parse().expect("Non-float value in svg points");
            points_nopair.push(point);
        }
        let mut i: usize = 0;
        while i < points_nopair.len() {
            let x = points_nopair[i];
            let y = -points_nopair[i + 1]; // invert y
            i += 2;

            let vertex = Vertex { x, y };

            // ensure that vertex is not already in points
            if points.contains(&vertex) {
                println!(
                    "warning [mesh]: duplicate point at {:?} in polyline id {:?}",
                    &vertex,
                    polyline.id()
                );
                continue;
            }
            // ensure vertex is proper distance away from last point
            if let Some(last_vertex) = points.last() {
                let distance = f64::sqrt(
                    f64::powi(last_vertex.x - vertex.x, 2) + f64::powi(last_vertex.y - vertex.y, 2),
                );
                if distance < min_element_length.into() {
                    skipped_vertices += 1;
                    continue;
                }
            }

            points.push(Vertex { x, y });
        }

        // Save points to corresponding field
        let mut item_id: Option<&str> = None;

        if let Some(id) = polyline.attribute("id") {
            item_id = Some(id);
        }
        // try to resolve id from parent
        else if let Some(parent) = polyline.parent() {
            if let Some(id) = parent.attribute("id") {
                item_id = Some(id);
            }
        }

        if let Some(id) = item_id {
            if id.trim().starts_with("INNER") {
                vertex_containers.push(points)
            } else if id.trim().starts_with("OUTER") {
                if vertex_containers[0].is_empty() {
                    vertex_containers[0] = points
                } else {
                    return Err(MagnetiteError::Input(
                        "Multiple OUTER geometries in SVG".to_owned(),
                    ));
                }
            } else {
                println!("warning: skipping polyline geometry with id {id}. Only supports OUTER and INNER");
            }
        } else {
            return Err(MagnetiteError::Input(
                "Error in svg file. Missing id field on polyline".to_owned(),
            ));
        }
    }

    // Parse rectangles from svg xml
    let doc = roxmltree::Document::parse(&contents).unwrap();
    let rectangles: Vec<roxmltree::Node> = doc
        .descendants()
        .into_iter()
        .filter(|n| n.tag_name().name() == "rect")
        .collect();

    for rect in rectangles {
        let x: f64 = match rect.attribute("x") {
            Some(x) => x
                .parse()
                .expect(format!("Non-float value in svg points at node {:?}", rect.id()).as_str()),
            None => {
                println!(
                    "warning [mesh]: Missing x definition in rectangle {:?}. Assuming zero.",
                    rect.id()
                );
                0.0
            }
        };

        let y: f64 = match rect.attribute("y") {
            Some(y) => y
                .parse()
                .expect(format!("Non-float value in svg points at node {:?}", rect.id()).as_str()),
            None => {
                println!(
                    "warning [mesh]: Missing y definition in rectangle {:?}. Assuming zero.",
                    rect.id()
                );
                0.0
            }
        };

        let width: f64 = match rect.attribute("width") {
            Some(width) => width,
            None => {
                return Err(MagnetiteError::Input(format!(
                    "Error in svg file. No width definition in rectangle. Conflicting node: {:?}",
                    rect.id()
                )));
            }
        }
        .parse()
        .expect("Non-float value in svg points");
        let height: f64 = match rect.attribute("height") {
            Some(height) => height,
            None => {
                return Err(MagnetiteError::Input(format!(
                    "Error in svg file. No height definition in rectangle. Conflicting node: {:?}",
                    rect.id()
                )));
            }
        }
        .parse()
        .expect("Non-float value in svg points");

        let vertices = vec![
            Vertex { x: x, y: -y },
            Vertex {
                x: x + width,
                y: -y,
            },
            Vertex {
                x: x + width,
                y: -y - height,
            },
            Vertex { x, y: -y - height },
        ];

        // Save points to corresponding field
        let mut item_id: Option<&str> = None;

        if let Some(id) = rect.attribute("id") {
            item_id = Some(id);
        }
        // try to resolve id from parent
        else if let Some(parent) = rect.parent() {
            if let Some(id) = parent.attribute("id") {
                item_id = Some(id);
            }
        }

        if let Some(id) = item_id {
            if id.trim().starts_with("INNER") {
                vertex_containers.push(vertices)
            } else if id.trim().starts_with("OUTER") {
                if vertex_containers[0].is_empty() {
                    vertex_containers[0] = vertices
                } else {
                    return Err(MagnetiteError::Input(
                        "Multiple OUTER geometries in SVG".to_owned(),
                    ));
                }
            } else {
                println!("warning: skipping polyline geometer with id {id}. Only supports OUTER and INNER")
            }
        } else {
            return Err(MagnetiteError::Input(
                "Error in svg file. Missing id field on polyline".to_owned(),
            ));
        }
    }

    if skipped_vertices > 0 {
        println!("warning [mesh]: skipped {} vertices", skipped_vertices);
    }

    if vertex_containers[0].is_empty() {
        return Err(MagnetiteError::Input("No OUTER geometry".to_owned()));
    }

    Ok(vertex_containers)
}

/// Parses a CSV file into a list of vertices
///
/// # Arguments
/// * `csv_file` - The path to the input csv file
///
/// # Returns
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
/// * `vertices` - The vector of vertices to parse into a geometry
/// * `output_file` - The output .geo file
fn build_geo(
    vertices_containers: &Vec<Vec<Vertex>>,
    output_file: &str,
    characteristic_length_min: f32,
    characteristic_length_max: f32,
) -> Result<(), MagnetiteError> {
    let mut geo_file = std::fs::File::create(output_file).expect("Failed to create .geo file");

    // Define outer points
    geo_file
        .write("// Define outer points\n".as_bytes())
        .unwrap();
    for (i, vertex) in vertices_containers[0].iter().enumerate() {
        geo_file
            .write(format!("Point({}) = {{ {}, {}, 0, 1.0 }};\n", i, vertex.x, vertex.y).as_bytes())
            .unwrap();
    }

    // Define inner points
    geo_file
        .write("\n// Define inner points\n".as_bytes())
        .unwrap();

    let mut offset_counter: usize = vertices_containers[0].len();
    let mut inner_offsets: Vec<usize> =
        Vec::with_capacity(std::mem::size_of::<usize>() * (vertices_containers.len() - 1));

    inner_offsets.push(0);

    for vertices in vertices_containers[1..].iter() {
        inner_offsets.push(offset_counter);

        for (i, vertex) in vertices.iter().enumerate() {
            geo_file
                .write(
                    format!(
                        "Point({}) = {{ {}, {}, 0, 1.0 }};\n",
                        i + offset_counter,
                        vertex.x,
                        vertex.y
                    )
                    .as_bytes(),
                )
                .unwrap();
        }

        offset_counter += vertices.len();
    }

    // Connect points
    geo_file.write("\n// Connect points\n".as_bytes()).unwrap();

    for (i, vertices) in vertices_containers.iter().enumerate() {
        geo_file
            .write(format!("\n// Point connections for surface {i}\n").as_bytes())
            .unwrap();

        let point_offset = inner_offsets[i];

        for i in 1..vertices.len() {
            geo_file
                .write(
                    format!(
                        "Line({}) = {{ {}, {} }};\n",
                        i + point_offset - 1,
                        i + point_offset - 1,
                        i + point_offset
                    )
                    .as_bytes(),
                )
                .unwrap();
        }
        geo_file
            .write(
                format!(
                    "Line({}) = {{ {}, {} }};\n",
                    vertices.len() + point_offset - 1,
                    vertices.len() + point_offset - 1,
                    point_offset
                )
                .as_bytes(),
            )
            .unwrap();
    }

    // Define loops
    geo_file.write("\n//Register loops\n".as_bytes()).unwrap();

    for (i, vertices) in vertices_containers.iter().enumerate() {
        let point_offset = inner_offsets[i];

        geo_file
            .write(format!("Line Loop({}) = {{", i + 1).as_bytes())
            .unwrap();
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
                        i + point_offset
                    )
                    .as_bytes(),
                )
                .unwrap();
        }
        geo_file.write(" };\n".as_bytes()).unwrap();
    }

    geo_file.write("\n//Define surface\n".as_bytes()).unwrap();

    geo_file.write("Plane Surface(1) = {".as_bytes()).unwrap();

    let iter: Vec<usize> = {
        if vertices_containers.len() > 2 {
            (0..vertices_containers.len()).collect()
        } else {
            (0..vertices_containers.len()).rev().collect()
        }
    };

    for (i, loop_idx) in iter.iter().enumerate() {
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
                    loop_idx + 1
                )
                .as_bytes(),
            )
            .unwrap();
    }
    geo_file.write(" };\n".as_bytes()).unwrap();

    // Define meshing settings
    geo_file
        .write(
            format!(
                "\n// Define Mesh Settings\n\
                Mesh.ElementOrder = 1;\n\
                Mesh.Algorithm  = 1;\n\
                Mesh.CharacteristicLengthMin = {cl_min};\n\
                Mesh.CharacteristicLengthMax = {cl_max};\n\
                Mesh 2;\n\
                ",
                cl_min = characteristic_length_min,
                cl_max = characteristic_length_max,
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
    vertices: &Vec<Vec<Vertex>>,
    output: &str,
    characteristic_length_min: f32,
    characteristic_length_max: f32,
) -> Result<(), MagnetiteError> {
    let geo_filepath = "geom.geo";

    println!(
        "info: building .geo for Gmsh with {:.3}< CL < {:.3}",
        characteristic_length_min, characteristic_length_max
    );
    build_geo(
        vertices,
        geo_filepath,
        characteristic_length_min,
        characteristic_length_max,
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

/// Parses a .msh file into Nodes and Elements
///
/// # Arguments
/// * `mesh_file` - The path to the mesh file
///
/// # Returns
/// A tuple with a vector of the parsed nodes and a vector of the parsed
/// elements, in that order.
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

                let mut node_tags: Vec<usize> =
                    Vec::with_capacity(num_nodes_local * std::mem::size_of::<usize>());

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

/// Parses the input json into a JsonValue object
///
/// # Arguments
/// * `input_file` - The path to the input file
///
/// # Returns
/// A JsonValue object
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
            "Input json missing metadata field".to_string(),
        ));
    }
    if !input_file_json.has_key("boundary_conditions") {
        return Err(MagnetiteError::Input(
            "Input json missing boundary_conditions field in metadata section".to_string(),
        ));
    }
    if !input_file_json["metadata"].has_key("part_thickness") {
        return Err(MagnetiteError::Input(
            "Input json missing part_thickness field in metadata section".to_string(),
        ));
    }
    if !input_file_json["metadata"].has_key("material_elasticity") {
        return Err(MagnetiteError::Input(
            "Input json missing material_elasticity field in metadata section".to_string(),
        ));
    }
    if !input_file_json["metadata"].has_key("poisson_ratio") {
        return Err(MagnetiteError::Input(
            "Input json missing poisson_ratio field in metadata section".to_string(),
        ));
    }

    Ok(input_file_json)
}

/// Parses Model Metadata from the input_json
///
/// # Arguments
/// * `input_json`: The input file as a JsonValue object
///
/// # Returns
/// A ModelMetadata instance
fn parse_input_metadata(input_json: &JsonValue) -> Result<ModelMetadata, MagnetiteError> {
    let youngs_modulus = input_json["metadata"]["material_elasticity"].as_f64();

    let part_thickness = input_json["metadata"]["part_thickness"].as_f64();

    let poisson_ratio = input_json["metadata"]["poisson_ratio"].as_f64();

    let characteristic_length_min = input_json["metadata"]["characteristic_length_min"].as_f32();

    let characteristic_length_max = input_json["metadata"]["characteristic_length_max"].as_f32();

    if youngs_modulus.is_none() {
        return Err(MagnetiteError::Input(
            "Input json missing material elasticity".to_owned(),
        ));
    }
    if poisson_ratio.is_none() {
        return Err(MagnetiteError::Input(
            "Input json missing poisson ratio".to_owned(),
        ));
    }
    if characteristic_length_min.is_none() {
        return Err(MagnetiteError::Input(
            "Input json missing minimum characteristic length".to_owned(),
        ));
    }
    if characteristic_length_max.is_none() {
        return Err(MagnetiteError::Input(
            "Input json missing maximum characteristic length".to_owned(),
        ));
    }

    Ok(ModelMetadata {
        youngs_modulus: youngs_modulus.unwrap(),
        poisson_ratio: poisson_ratio.unwrap(),
        part_thickness: part_thickness.unwrap(),
        characteristic_length_min: characteristic_length_min.unwrap(),
        characteristic_length_max: characteristic_length_max.unwrap(),
    })
}

/// Applies boundary conditions to a vector of nodes from the input json
///
/// # Arguments
/// * `input_json` - The input file as a JsonValue object
/// * `nodes` - A mutable reference to the vector of nodes
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
            boundary_region.y_min = rule_json["region"]["y_target_min"]
                .as_f64()
                .expect(format!("Bad value for y_target_min in {name}").as_str())
        }
        if rule_json["region"].has_key("y_target_max") {
            boundary_region.y_max = rule_json["region"]["y_target_max"]
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

        // Validate input
        if boundary_region.x_min > boundary_region.x_max {
            return Err(MagnetiteError::Input(format!(
                "Boundary '{name}' has x_target_min greater than x_target_max"
            )));
        }
        if boundary_region.y_min > boundary_region.y_max {
            return Err(MagnetiteError::Input(format!(
                "Boundary '{name}' has y_target_min greater than y_target_max"
            )));
        }
        if boundary_target.fx.is_none() && boundary_target.ux.is_none() {
            return Err(MagnetiteError::Input(format!(
                "Boundary '{name}' is under-constrained in x-axis"
            )));
        }
        if boundary_target.fy.is_none() && boundary_target.uy.is_none() {
            return Err(MagnetiteError::Input(format!(
                "Boundary '{name}' is under-constrained in y-axis"
            )));
        }
        if boundary_target.fx.is_some() && boundary_target.ux.is_some() {
            return Err(MagnetiteError::Input(format!(
                "Boundary '{name}' is over-constrained in x-axis"
            )));
        }
        if boundary_target.fy.is_some() && boundary_target.uy.is_some() {
            return Err(MagnetiteError::Input(format!(
                "Boundary '{name}' is over-constrained in y-axis"
            )));
        }

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
    geometry_files: Vec<&str>,
    input_file: &str,
) -> Result<(Vec<Node>, Vec<Element>, ModelMetadata), MagnetiteError> {
    let input_file_json = load_input_file(input_file)?;
    let model_metadata = parse_input_metadata(&input_file_json)?;

    let mut vertices: Vec<Vec<Vertex>> = Vec::new();

    for geom in geometry_files {
        if geom.ends_with(".svg") {
            vertices = parse_svg(geom, model_metadata.characteristic_length_min)?;
            break;
        } else if geom.ends_with(".csv") {
            vertices.push(parse_csv(geom)?);
        } else {
            return Err(MagnetiteError::Input(
                format!("Unrecognized geometry filetype {geom}").to_string(),
            ));
        }
    }

    let mesh_filepath = "geom.msh";
    compute_mesh(
        &vertices,
        mesh_filepath,
        model_metadata.characteristic_length_min,
        model_metadata.characteristic_length_max,
    )?;

    let (mut nodes, elements) = parse_mesh(mesh_filepath)?;

    apply_boundary_conditions(&input_file_json, &mut nodes)?;

    Ok((nodes, elements, model_metadata))
}
