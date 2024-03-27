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
                "Unable to find svg file {}",
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
