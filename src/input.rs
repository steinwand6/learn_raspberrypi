use rppal::gpio::Gpio;
use std::error::Error;

const GPIO24: u8 = 24;
const GPIO23: u8 = 23;
const GPIO18: u8 = 18;
const SPIMOSI: u8 = 10;
const GPIO22: u8 = 22;
const GPIO27: u8 = 27;
const GPIO17: u8 = 17;
const GPIO25: u8 = 25;

pub fn button() -> Result<(), Box<dyn Error>> {
    let mut input = Gpio::new()?.get(GPIO18)?.into_input();
    let mut output = Gpio::new()?.get(GPIO17)?.into_output();

    output.set_high();
    input
        .set_interrupt(rppal::gpio::Trigger::FallingEdge)
        .expect("failed to set_interrupt.");
    loop {
        match input.poll_interrupt(true, None) {
            Ok(_) => output.toggle(),
            Err(e) => println!("{}", e),
        }
    }
}
