use std::fmt::Display;

#[derive(Debug)]
pub enum MagnetiteError {
    Input(String),
    Mesher(String),
    Solver(String),
    PostProcessor(String),
}

impl Display for MagnetiteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (err_name, value) = match self {
            MagnetiteError::Input(v) => ("Input", v),
            MagnetiteError::Mesher(v) => ("Mesher", v),
            MagnetiteError::Solver(v) => ("Solver", v),
            MagnetiteError::PostProcessor(v) => ("Post Processor", v),
        };

        write!(f, "{} error: {}", err_name, value)
    }
}
