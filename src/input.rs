use rppal::gpio::{Gpio, InputPin, Level, OutputPin};
use rppal::system::Error;
use std::collections::HashSet;
use std::f64::INFINITY;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

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

struct Adc0834 {
    adc_cs: u8,
    adc_do: u8,
    adc_di: u8,
    adc_clk: u8,
}

impl Adc0834 {
    fn new(adc_cs: u8, adc_do: u8, adc_di: u8, adc_clk: u8) -> Self {
        Self {
            adc_cs,
            adc_do,
            adc_di,
            adc_clk,
        }
    }
    fn get_adc_result(&self, ch_pin: u8) -> Result<u8, Box<dyn Error>> {
        let mut adc_cs = Gpio::new()?.get(self.adc_cs)?.into_output();
        let mut adc_do = Gpio::new()?.get(self.adc_do)?.into_input();
        let mut adc_di = Gpio::new()?.get(self.adc_di)?.into_output();
        let mut adc_clk = Gpio::new()?.get(self.adc_clk)?.into_output();

        // 変換の開始
        adc_cs.set_low();
        // スタートビット
        snd_bit(&mut adc_clk, &mut adc_di, 1);
        // SGL
        snd_bit(&mut adc_clk, &mut adc_di, 1);
        // ODD
        snd_bit(&mut adc_clk, &mut adc_di, ch_pin & 1 as u8);
        // Select
        snd_bit(&mut adc_clk, &mut adc_di, if ch_pin > 1 { 1 } else { 0 });

        // スカされるクロック。送信する値に意味なし。
        snd_bit(&mut adc_clk, &mut adc_di, 1);

        let mut msb = 0;
        let mut lsb;
        // MSB-First Data
        for _ in 0..7 {
            msb = msb << 1 | rcv_bit(&mut adc_clk, &mut adc_do);
        }
        // MSB-First DataとLSB-First Dataの最下位bit
        adc_clk.set_low();
        thread::sleep(Duration::from_micros(2));
        msb = msb << 1 | if adc_do.is_high() { 1 as u8 } else { 0 as u8 };
        lsb = if adc_do.is_high() { 1 } else { 0 };
        adc_clk.set_high();
        // LSB-First Data
        for i in 1..8 {
            lsb = rcv_bit(&mut adc_clk, &mut adc_do) << i | lsb;
        }
        // 変換の終了
        adc_cs.set_high();
        if lsb == msb {
            Ok(lsb)
        } else {
            Ok(0)
        }
    }
}

pub fn joystick() -> Result<(), Box<dyn Error>> {
    let button = Gpio::new()?.get(GPIO22)?.into_input_pullup();

    let mut x_val;
    let mut y_val;
    let adc = Adc0834::new(GPIO17, GPIO23, GPIO27, GPIO18);

    loop {
        x_val = adc.get_adc_result(0)?;
        y_val = adc.get_adc_result(1)?;
        println!(
            "x: {}, y: {}, button: {}",
            x_val,
            y_val,
            if button.is_low() {
                "pressed"
            } else {
                "not pressed"
            }
        );
        thread::sleep(Duration::from_millis(100));
    }
}

pub fn photoregister() -> Result<(), Box<dyn Error>> {
    let mut val;

    let adc = Adc0834::new(GPIO17, GPIO23, GPIO27, GPIO18);
    let mut led = Gpio::new()?.get(GPIO22)?.into_output();

    loop {
        val = adc.get_adc_result(0)?;
        led.set_pwm_frequency(2000.0, (val as f64) / 255.0)?;
        println!("val: {}", val);
        thread::sleep(Duration::from_millis(100));
    }
}

pub fn thermistor() -> Result<(), Box<dyn Error>> {
    use num::Float;

    let mut analog_val: u8;
    let mut vr: f64;
    let mut rt: f64;
    let mut temp: f64;
    let mut cel: f64;
    let mut fah: f64;
    let mut last_cel: f64 = INFINITY;

    let adc = Adc0834::new(GPIO17, GPIO23, GPIO27, GPIO18);

    loop {
        analog_val = adc.get_adc_result(0).expect("adc failed");
        vr = 5.0 * analog_val as f64 / 255.0;
        rt = 10000.0 * vr / (5.0 - vr);
        temp = 1.0 / ((Float::ln(rt / 10000.0) / 3950.0) + (1.0 / (273.15 + 25.0)));
        cel = temp - 273.15;
        fah = cel * 1.8 + 32.0;
        if last_cel != cel {
            println!("cel: {}, fah: {}", cel, fah);
        }
        last_cel = cel;
        thread::sleep(Duration::from_millis(100));
    }
}

struct Dht11 {
    pin: u8,
}

#[derive(Debug)]
enum Dth11Error {
    GpioError(rppal::gpio::Error),
    TimeOut,
    CheckSum,
}

impl From<rppal::gpio::Error> for Dth11Error {
    fn from(e: rppal::gpio::Error) -> Dth11Error {
        Dth11Error::GpioError(e)
    }
}

impl Dht11 {
    pub fn new(pin: u8) -> Self {
        Dht11 { pin }
    }

    pub fn read(&self) -> Result<((u8, u8), (u8, u8)), Dth11Error> {
        // send init request
        {
            let mut output = Gpio::new()?.get(self.pin)?.into_output();
            output.set_low();
            thread::sleep(Duration::from_millis(18));
            output.set_high();
            thread::sleep(Duration::from_nanos(40));
        }
        // get data from sensor
        let mut bytes = [0u8; 5];
        {
            let input = Gpio::new()?.get(self.pin)?.into_input();
            self.wait_level(&input, Level::High);
            self.wait_level(&input, Level::Low);
            self.wait_level(&input, Level::High);
            for b in bytes.iter_mut() {
                for _ in 0..8 {
                    *b <<= 1;
                    self.wait_level(&input, Level::Low);
                    let dur = self.wait_level(&input, Level::High)?;
                    if dur > 16 {
                        *b |= 1;
                    }
                }
            }
        }

        let sum: u16 = bytes.iter().take(4).map(|b| *b as u16).sum();
        if bytes[4] as u16 == sum & 0x00FF {
            return Ok(((bytes[0], bytes[1]), (bytes[2], bytes[3])));
        } else {
            return Err(Dth11Error::CheckSum);
        }
    }

    fn wait_level(&self, input_pin: &InputPin, level: Level) -> Result<u8, Dth11Error> {
        for i in 0u8..255 {
            if input_pin.read() == level {
                return Ok(i);
            }
            thread::sleep(Duration::from_micros(1));
        }
        Err(Dth11Error::TimeOut)
    }
}

pub fn dht() -> Result<(), Dth11Error> {
    let mut dht11 = Dht11::new(GPIO17);
    loop {
        let ((h1, h2), (t1, t2)) = dht11.read()?;
        println!("h: {}.{}%  t: {}.{}*c", h1, h2, t1, t2);
    }
}
