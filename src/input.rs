use rppal::gpio::{Gpio, InputPin, OutputPin};
use std::collections::HashSet;
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

fn snd_bit(clk_pin: &mut OutputPin, input_pin: &mut OutputPin, value: u8) {
    clk_pin.set_low();
    thread::sleep(Duration::from_micros(2));
    if value == 0 {
        input_pin.set_low();
    } else {
        input_pin.set_high();
    }
    clk_pin.set_high();
    thread::sleep(Duration::from_micros(2));
}

fn rcv_bit(clk_pin: &mut OutputPin, output_pin: &mut InputPin) -> u8 {
    let result;
    clk_pin.set_low();
    thread::sleep(Duration::from_micros(2));
    if output_pin.is_high() {
        result = 1;
    } else {
        result = 0;
    }
    clk_pin.set_high();
    thread::sleep(Duration::from_micros(2));
    return result;
}

pub fn potentiometer() -> Result<(), Box<dyn Error>> {
    let mut msb = 0;
    let mut lsb = 0;

    let mut adc_cs = Gpio::new()?.get(GPIO17)?.into_output();
    let mut adc_do = Gpio::new()?.get(GPIO23)?.into_input();
    let mut adc_di = Gpio::new()?.get(GPIO27)?.into_output();
    let mut adc_clk = Gpio::new()?.get(GPIO18)?.into_output();
    let mut led = Gpio::new()?.get(GPIO22)?.into_output();

    loop {
        // 変換の開始
        adc_cs.set_low();
        // スタートビット
        snd_bit(&mut adc_clk, &mut adc_di, 1);
        // SGL
        snd_bit(&mut adc_clk, &mut adc_di, 1);
        // ODD
        snd_bit(&mut adc_clk, &mut adc_di, 1);
        // Select
        snd_bit(&mut adc_clk, &mut adc_di, 0);

        // スカされるbit。送信する値に意味なし。
        snd_bit(&mut adc_clk, &mut adc_di, 1);

        // MSB-First Data
        for _ in 0..7 {
            msb = msb << 1 | rcv_bit(&mut adc_clk, &mut adc_do);
        }
        // MSB-First DataとLSB-First Dataの最下位bit
        adc_clk.set_low();
        thread::sleep(Duration::from_micros(2));
        msb = msb << 1 | if adc_do.is_high() { 1 } else { 0 };
        lsb = if adc_do.is_high() { 1 } else { 0 };
        adc_clk.set_high();
        // LSB-First Data
        for i in 1..8 {
            lsb = rcv_bit(&mut adc_clk, &mut adc_do) << i | lsb;
        }
        // 変換の終了
        adc_cs.set_high();
        // LED点灯
        if msb == lsb {
            led.set_pwm_frequency(2000.0, (msb as f64) / 255.0)?;
        }
    }
}

pub fn keypad() -> Result<(), Box<dyn Error>> {
    const KEYS: [char; 16] = [
        '1', '2', '3', 'A', '4', '5', '6', 'B', '7', '8', '9', 'C', '*', '0', '#', 'D',
    ];
    let row = [GPIO18, GPIO23, GPIO24, GPIO25];
    let col = [SPIMOSI, GPIO22, GPIO27, GPIO17];
    let mut row_pins = row
        .map(|pin| -> Result<OutputPin, Box<dyn Error>> {
            Ok(Gpio::new()?.get(pin)?.into_output())
        })
        .map(|result: Result<OutputPin, Box<dyn Error>>| result.unwrap());
    let col_pins = col
        .map(|pin| -> Result<InputPin, Box<dyn Error>> { Ok(Gpio::new()?.get(pin)?.into_input()) })
        .map(|result: Result<InputPin, Box<dyn Error>>| result.unwrap());

    let mut pressed: HashSet<char> = vec![].into_iter().collect();
    let mut last_pressed: HashSet<char> = vec![].into_iter().collect();
    loop {
        for (i, output) in &mut row_pins.iter_mut().enumerate() {
            output.set_high();
            for (j, input) in col_pins.iter().enumerate() {
                if input.is_high() {
                    pressed.insert(KEYS[i * 4 + j]);
                }
            }
            thread::sleep(Duration::from_millis(1));
            output.set_low();
        }
        if pressed.difference(&last_pressed).collect::<Vec<_>>().len() != 0 {
            println!("{:?}", pressed.difference(&last_pressed));
        }
        last_pressed.clone_from(&pressed);
        pressed.clear();
        thread::sleep(Duration::from_millis(100));
    }
}
