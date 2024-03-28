mod datatypes;
mod error;
mod mesher;

fn main() {
    mesher::run("vertices.csv", 15.0, 5.0).unwrap();
}
