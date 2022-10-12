use rppal::gpio::{Gpio, OutputPin};
use std::time::Duration;
use std::{error::Error, thread};

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

pub fn slide_button() -> Result<(), Box<dyn Error>> {
    let mut led_1 = Gpio::new()?.get(GPIO22)?.into_output();
    let mut led_2 = Gpio::new()?.get(GPIO27)?.into_output();
    let input_pin = Gpio::new()?.get(GPIO17)?.into_input();

    loop {
        if input_pin.is_high() {
            led_1.set_low();
            led_2.set_high();
            println!("LED1 on");
            thread::sleep(Duration::from_secs(1));
        } else {
            led_2.set_low();
            led_1.set_high();
            println!("LED2 on");
            thread::sleep(Duration::from_secs(1));
        }
    }
}
