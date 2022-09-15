pub mod output;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    output::rgb_led()
}
