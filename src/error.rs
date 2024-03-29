#[derive(Debug)]
pub enum MagnetiteError {
    Input(String),
    Mesher(String),
    Solver(String),
}
