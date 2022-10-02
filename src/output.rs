use std::error::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use rppal::gpio::{Gpio, OutputPin};
use rppal::system::DeviceInfo;

const GPIO24: u8 = 24;
const GPIO23: u8 = 23;
const GPIO18: u8 = 18;
const SPIMOSI: u8 = 10;
const GPIO22: u8 = 22;
const GPIO27: u8 = 27;
const GPIO17: u8 = 17;

pub fn blink_led() -> Result<(), Box<dyn Error>> {
    println!("Blinking an LED on a {}.", DeviceInfo::new()?.model());

    const GPIO_LED: u8 = 0;
    let mut pin = Gpio::new()?.get(GPIO_LED)?.into_output();

    // Blink the LED by setting the pin's logic level high for 500 ms.
    for _ in 0..10 {
        pin.set_low();
        println!("...LED on");
        thread::sleep(Duration::from_millis(500));
        pin.set_high();
        println!("...LED off");
        thread::sleep(Duration::from_millis(500));
    }
    Ok(())
}

pub fn rgb_led() -> Result<(), Box<dyn Error>> {
    const COLOR_FLAGS: [u64; 7] = [0b000, 0b100, 0b010, 0b001, 0b110, 0b101, 0b011];
    const GPIO_LED_RED: u8 = 17;
    const GPIO_LED_GREEN: u8 = 18;
    const GPIO_LED_BLUE: u8 = 27;

    const DURATION: u64 = 100 * 100;

    // Retrieve the GPIO pin and configure it as an output.
    let mut pin1 = Gpio::new()?.get(GPIO_LED_RED)?.into_output();
    let mut pin2 = Gpio::new()?.get(GPIO_LED_GREEN)?.into_output();
    let mut pin3 = Gpio::new()?.get(GPIO_LED_BLUE)?.into_output();

    let light_led = |pin: &mut OutputPin, palse_width: u64| {
        pin.set_pwm(
            Duration::from_micros(DURATION),
            Duration::from_micros(palse_width),
        )
    };
    loop {
        for flags in COLOR_FLAGS {
            light_led(&mut pin1, (flags & 0b100) * 100)?;
            light_led(&mut pin2, (flags & 0b010) * 100)?;
            light_led(&mut pin3, (flags & 0b001) * 100)?;
            thread::sleep(Duration::from_millis(500));
        }
    }
    Ok(())
}

fn turn_high_and_low(pin: &mut OutputPin, duration: Duration) {
    pin.set_high();
    thread::sleep(duration);
    pin.set_low();
}

pub fn segment7() -> Result<(), Box<dyn Error>> {
    let seg_code: [u8; 16] = [
        0x3f, 0x06, 0x5b, 0x4f, 0x66, 0x6d, 0x7d, 0x07, 0x7f, 0x6f, 0x77, 0x7c, 0x39, 0x5e, 0x79,
        0x71,
    ];

    let mut pin_sdi = Gpio::new()?.get(GPIO17)?.into_output();
    let mut pin_rclk = Gpio::new()?.get(GPIO18)?.into_output();
    let mut pin_srclk = Gpio::new()?.get(GPIO27)?.into_output();

    pin_sdi.set_low();
    pin_rclk.set_low();
    pin_srclk.set_low();

    for code in seg_code {
        for i in 0..8 {
            if 0x80 & (code << i) != 0 {
                pin_sdi.set_high();
            } else {
                pin_sdi.set_low();
            }
            turn_high_and_low(&mut pin_srclk, Duration::from_millis(1));
        }
        turn_high_and_low(&mut pin_rclk, Duration::from_millis(1000));
    }
    clear_display(&mut pin_sdi, &mut pin_rclk, &mut pin_srclk, true);
    Ok(())
}

fn clear_display(sdi: &mut OutputPin, rclk: &mut OutputPin, srclk: &mut OutputPin, is_anode: bool) {
    if is_anode {
        sdi.set_low();
    } else {
        sdi.set_high();
    }
    for _ in 0..8 {
        turn_high_and_low(srclk, Duration::from_millis(0));
    }
    turn_high_and_low(rclk, Duration::from_millis(0));
}

fn hc595_shift(sdi: &mut OutputPin, rclk: &mut OutputPin, srclk: &mut OutputPin, code: u8) {
    for i in 0..8 {
        if 0x80 & (code << i) != 0 {
            sdi.set_high();
        } else {
            sdi.set_low();
        }
        turn_high_and_low(srclk, Duration::from_millis(0));
    }
    turn_high_and_low(rclk, Duration::from_millis(0));
}

pub fn four_digit_segment7() -> Result<(), Box<dyn Error>> {
    let timer = timer::Timer::new();
    let count = Arc::new(Mutex::new(0));

    let guard = {
        let count = count.clone();
        timer.schedule_repeating(chrono::Duration::milliseconds(100), move || {
            *count.lock().unwrap() += 1;
        })
    };

    let seg_code: [u8; 10] = [0xc0, 0xf9, 0xa4, 0xb0, 0x99, 0x92, 0x82, 0xf8, 0x80, 0x90];

    let mut pin_sdi = Gpio::new()?.get(GPIO24)?.into_output();
    let mut pin_rclk = Gpio::new()?.get(GPIO23)?.into_output();
    let mut pin_srclk = Gpio::new()?.get(GPIO18)?.into_output();

    let mut place_pins: [OutputPin; 4] = [
        Gpio::new()?.get(SPIMOSI)?.into_output(),
        Gpio::new()?.get(GPIO22)?.into_output(),
        Gpio::new()?.get(GPIO27)?.into_output(),
        Gpio::new()?.get(GPIO17)?.into_output(),
    ];

    let mut pick_digit = |digit: usize| {
        for p in &mut place_pins {
            p.set_low();
        }
        place_pins[digit].set_high();
    };

    let mut light_1digit = |count: usize, digit: usize| {
        let base: i32 = 10;
        clear_display(&mut pin_sdi, &mut pin_rclk, &mut pin_srclk, false);
        pick_digit(digit);
        hc595_shift(
            &mut pin_sdi,
            &mut pin_rclk,
            &mut pin_srclk,
            seg_code[count / (base.pow(digit as u32) as usize) % 10],
        );
    };
    while *count.lock().unwrap() < 10000 {
        light_1digit(*count.lock().unwrap(), 0);
        light_1digit(*count.lock().unwrap(), 1);
        light_1digit(*count.lock().unwrap(), 2);
        light_1digit(*count.lock().unwrap(), 3);
    }
    clear_display(&mut pin_sdi, &mut pin_rclk, &mut pin_srclk, false);
    drop(guard);
    Ok(())
}

fn hc595_in(sdi: &mut OutputPin, srclk: &mut OutputPin, code: u8) {
    for i in 0..8 {
        if 0x80 & (code << i) != 0 {
            sdi.set_high();
        } else {
            sdi.set_low();
        }
        turn_high_and_low(srclk, Duration::from_millis(0));
    }
}

fn hc595_out(rclk: &mut OutputPin) {
    turn_high_and_low(rclk, Duration::from_millis(0));
}

pub fn light_led_dot_matrix() -> Result<(), Box<dyn Error>> {
    let code_h: [u8; 20] = [
        0x01, 0xff, 0x80, 0xff, 0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff,
    ];
    let code_l: [u8; 20] = [
        0x00, 0x7f, 0x00, 0xfe, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xfe, 0xfd, 0xfb,
        0xf7, 0xef, 0xdf, 0xbf, 0x7f,
    ];
    let mut pin_sdi = Gpio::new()?.get(GPIO17)?.into_output();
    let mut pin_rclk = Gpio::new()?.get(GPIO18)?.into_output();
    let mut pin_srclk = Gpio::new()?.get(GPIO27)?.into_output();

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");
    while running.load(Ordering::SeqCst) {
        for i in 0..code_h.len() {
            hc595_in(&mut pin_sdi, &mut pin_srclk, code_l[i]);
            hc595_in(&mut pin_sdi, &mut pin_srclk, code_h[i]);
            hc595_out(&mut pin_rclk);
            thread::sleep(Duration::from_millis(100));
        }
        for i in (0..code_h.len()).rev() {
            hc595_in(&mut pin_sdi, &mut pin_srclk, code_l[i]);
            hc595_in(&mut pin_sdi, &mut pin_srclk, code_h[i]);
            hc595_out(&mut pin_rclk);
            thread::sleep(Duration::from_millis(100));
        }
    }
    clear_display(&mut pin_sdi, &mut pin_rclk, &mut pin_srclk, true);
    Ok(())
}

pub fn beep_active_buzzer() -> Result<(), Box<dyn Error>> {
    let mut beep_pin = Gpio::new()?.get(GPIO17)?.into_output();

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    while running.load(Ordering::SeqCst) {
        beep_pin.set_low();
        thread::sleep(Duration::from_millis(100));
        beep_pin.set_high();
        thread::sleep(Duration::from_millis(10));
    }
    beep_pin.set_high();
    Ok(())
}

pub fn beep_passive_buzzer() -> Result<(), Box<dyn Error>> {
    struct CDEFGAB {
        c: f64,
        d: f64,
        e: f64,
        f: f64,
        g: f64,
        a: f64,
        b: f64,
    }
    const L_TONE: CDEFGAB = CDEFGAB {
        c: 130.813,
        d: 146.832,
        e: 164.814,
        f: 174.614,
        g: 195.998,
        a: 220.0,
        b: 246.942,
    };
    const M_TONE: CDEFGAB = CDEFGAB {
        c: 261.626,
        d: 293.665,
        e: 329.628,
        f: 349.228,
        g: 391.995,
        a: 440.0,
        b: 493.883,
    };
    const H_TONE: CDEFGAB = CDEFGAB {
        c: 523.251,
        d: 587.33,
        e: 659.255,
        f: 698.456,
        g: 783.991,
        a: 880.0,
        b: 987.767,
    };

    // Retrieve the GPIO pin and configure it as an output.
    let mut beep_pin = Gpio::new()?.get(GPIO17)?.into_output();

    let mut beep = |tone: f64, duty_cycle: f64, millis: u64| {
        let _ = beep_pin.set_pwm_frequency(tone, duty_cycle);
        thread::sleep(Duration::from_millis(millis));
    };

    for i in 0..3 {
        // ときめく
        beep(M_TONE.b, 0.1, 500);
        beep(H_TONE.d, 0.1, 500);
        beep(M_TONE.a, 0.1, 500);
        beep(M_TONE.b, 0.1, 500);
        // こころの
        beep(M_TONE.a, 0.1, 500);
        beep(H_TONE.d, 0.1, 500);
        beep(H_TONE.d, 0.1, 480);
        beep(H_TONE.d, 0.0, 20);
        beep(H_TONE.e, 0.1, 500);
        if i == 0 {
            // もーしょん
            beep(739.938, 0.1, 500);
            beep(H_TONE.d, 0.1, 500);
            beep(M_TONE.b, 0.1, 500);
            beep(M_TONE.a, 0.1, 480);
            beep(M_TONE.a, 0.0, 20);
            // が
            beep(M_TONE.a, 0.1, 500);
            beep(M_TONE.a, 0.0, 1500);
        } else if i == 1 {
            // やまない
            beep(H_TONE.d, 0.1, 500);
            beep(H_TONE.e, 0.1, 500);
            beep(H_TONE.a, 0.1, 750);
            beep(H_TONE.a, 0.0, 250);
            // の
            beep(H_TONE.d, 0.1, 1000);
            beep(H_TONE.d, 0.0, 1000);
        } else {
            // プログラ
            beep(739.938, 0.1, 500);
            beep(H_TONE.d, 0.1, 500);
            beep(M_TONE.b, 0.1, 500);
            beep(M_TONE.a, 0.1, 480);
            beep(H_TONE.a, 0.0, 20);
            // ム しりた
            beep(M_TONE.a, 0.1, 480);
            beep(H_TONE.a, 0.0, 20);
            beep(M_TONE.a, 0.0, 500);
            beep(M_TONE.a, 0.1, 250);
            beep(M_TONE.b, 0.1, 250);
            beep(M_TONE.b, 0.0, 250);
            beep(H_TONE.d, 0.1, 250);
            // い しりた
            beep(H_TONE.d, 0.1, 500);
            beep(H_TONE.d, 0.0, 500);
            beep(M_TONE.a, 0.1, 250);
            beep(M_TONE.b, 0.1, 250);
            beep(M_TONE.b, 0.0, 250);
            beep(H_TONE.d, 0.1, 250);
            // い ねえもっと
            beep(H_TONE.d, 0.1, 250);
            beep(M_TONE.b, 0.0, 250);
            beep(M_TONE.a, 0.0, 500);
            beep(739.938, 0.1, 500);
            beep(H_TONE.d, 0.1, 500);
            // つきあって
            beep(H_TONE.d, 0.1, 500);
            beep(M_TONE.a, 0.1, 500);
            beep(H_TONE.e, 0.1, 750);
            beep(H_TONE.e, 0.08, 250);
            beep(H_TONE.d, 0.1, 1000);
        }
    }
    beep(H_TONE.d, 0.0, 1000);
    for i in 0..4 {
        // ダンスロボットダンス
        beep(H_TONE.d, 0.1, 200);
        beep(H_TONE.d, 0.0, 550);
        beep(H_TONE.d, 0.1, 230);
        beep(H_TONE.d, 0.01, 20);
        beep(H_TONE.d, 0.1, 200);
        beep(H_TONE.d, 0.0, 300);
        beep(554.365, 0.1, 250);
        beep(H_TONE.d, 0.1, 250);
        if i == 0 || i == 2 {
            beep(M_TONE.a, 0.0, 500);
            beep(M_TONE.a, 0.1, 500);
            beep(739.938, 0.1, 500);
            beep(H_TONE.d, 0.1, 450);
            beep(H_TONE.d, 0.0, 50);
        } else if i == 1 {
            beep(M_TONE.a, 0.0, 500);
            beep(H_TONE.d, 0.1, 500);
            beep(H_TONE.e, 0.1, 230);
            beep(H_TONE.e, 0.05, 20);
            beep(H_TONE.d, 0.1, 450);
            beep(H_TONE.d, 0.0, 50);
        } else {
            beep(H_TONE.d, 0.0, 500);
            beep(H_TONE.d, 0.1, 500);
            beep(H_TONE.a, 0.1, 250);
            beep(H_TONE.a, 0.0, 250);
            beep(H_TONE.d, 0.1, 500);
        }
    }
    Ok(())
}
