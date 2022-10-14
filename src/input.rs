use rppal::gpio::{Gpio, OutputPin};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
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
    let mut input_pin = Gpio::new()?.get(GPIO17)?.into_input();

    if input_pin.is_low() {
        led_1.set_low();
        led_2.set_high();
    } else {
        led_2.set_high();
        led_1.set_high();
    }

    input_pin
        .set_interrupt(rppal::gpio::Trigger::Both)
        .expect("failed to set_interrupt.");

    loop {
        match input_pin.poll_interrupt(true, None) {
            Ok(_) => {
                led_1.toggle();
                led_2.toggle();
            }
            Err(e) => println!("{}", e),
        }
    }
}

pub fn tilt() -> Result<(), Box<dyn Error>> {
    let mut led_1 = Gpio::new()?.get(GPIO22)?.into_output();
    let mut led_2 = Gpio::new()?.get(GPIO27)?.into_output();
    let mut input_pin = Gpio::new()?.get(GPIO17)?.into_input();

    input_pin
        .set_interrupt(rppal::gpio::Trigger::Both)
        .expect("failed to set_interrupt.");

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");
    while running.load(Ordering::SeqCst) {
        match input_pin.poll_interrupt(true, None) {
            Ok(trigger) => {
                thread::sleep(Duration::from_millis(10));
                match trigger {
                    Some(rppal::gpio::Level::High) => {
                        led_1.set_high();
                        led_2.set_low();
                    }
                    Some(rppal::gpio::Level::Low) => {
                        println!("Tilt!");
                        led_2.set_high();
                        led_1.set_low();
                    }
                    None => break,
                }
                thread::sleep(Duration::from_millis(500));
            }
            Err(_) => println!("\nEnd"),
        }
    }
    led_1.set_low();
    led_2.set_low();
    Ok(())
}
