mod datatypes;
mod error;
mod mesher;

fn main() {

    mesher::parse_svg("pyrite.svg");
}
