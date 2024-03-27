mod datatypes;
mod error;
mod mesher;

fn main() {

    for v in mesher::parse_csv("vertices.csv").expect("broken") {
        println!("{:?}", v);
    }
}
