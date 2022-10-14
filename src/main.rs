pub mod input;
pub mod output;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    input::tilt()
}
