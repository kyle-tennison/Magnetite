use crate::{datatypes::Vertex, error::MagnetiteError};


/// Parses a .svg file into a list of Vertexes
///
/// # Arguments
/// 
/// * `svg_file` - The path to the input svg file
/// 
/// # Returns
/// 
/// An ordered vector of Vertex instances
pub fn parse_svg(svg_file: &str) -> Result<Vec<Vertex>, MagnetiteError> {
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

    let polyline = match doc.descendants().find(|n| n.tag_name().name() == "polyline") {
        Some(p) => p,
        None => { return Err(MagnetiteError::Input("Error in svg file. No polyline element.".to_string()));}
    };

    let points_raw = match polyline.attribute("points") {
        Some(p) => p,
        None => {return Err(MagnetiteError::Input("Error in svg file. No points in polyline element.".to_string()))}
    }.split(" ");


    let mut points_nopair: Vec<f64> = Vec::new();

    for point_str in points_raw {
        let point: f64 = point_str.parse().expect("Non-float value in svg points");
        points_nopair.push(point);
    }

    let mut points: Vec<Vertex> = Vec::new();
    let mut i: usize = 0;
    while i < points_nopair.len() {
        let x = points_nopair[i];
        let y = points_nopair[i +1];
        
        points.push(Vertex{x, y});

        i += 2;
    }

    println!("info: successfully loaded {} vertices from svg", points.len());

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
pub fn parse_csv(csv_file: &str) -> Result<Vec<Vertex>, MagnetiteError> {

    let contents = match std::fs::read_to_string(csv_file) {
        Ok(c) => c,
        Err(_err) => {return Err(MagnetiteError::Input(format!("Unable to open csv file {}", csv_file)))} 
    };


    let mut headers: Vec<&str> = Vec::new();
    let mut x_index: usize = 0;
    let mut y_index: usize = 0;
    let mut vertices: Vec<Vertex> = Vec::new();

    for line in contents.split("\n"){

        if line.is_empty(){
            continue
        }
        
        if headers.len() == 0 {
            headers = line.split(",").map(|x| x.trim()).collect();

            if !headers.contains(&"x") || !headers.contains(&"y"){
                return Err(MagnetiteError::Input("Error in csv file: Missing x and/or y field".to_string()));
            }

            x_index = headers.iter().position(|f| f==&"x").unwrap();
            y_index = headers.iter().position(|f| f==&"y").unwrap();
        }
        else {

            let line_contents: Vec<f64> = line.split(",").map(|x| x.trim().parse().expect("Non-float value in csv points")).collect();

            let x = line_contents[x_index];
            let y = line_contents[y_index];

            vertices.push( Vertex{x, y});
            
        }
    }
    
    Ok(vertices)
}