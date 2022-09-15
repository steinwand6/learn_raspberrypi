use std::error::Error;
use std::thread;
use std::time::Duration;

use rppal::gpio::{Gpio, OutputPin};
use rppal::system::DeviceInfo;

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

    // 最初は周期100*100us, パルス幅0us
    light_led(&mut pin1, 0)?;
    light_led(&mut pin2, 0)?;
    light_led(&mut pin3, 0)?;
    thread::sleep(Duration::from_millis(500));

    // 赤
    light_led(&mut pin1, 100)?;
    light_led(&mut pin2, 0)?;
    light_led(&mut pin3, 0)?;
    thread::sleep(Duration::from_millis(500));

    // 緑
    light_led(&mut pin1, 0)?;
    light_led(&mut pin2, 100)?;
    light_led(&mut pin3, 0)?;
    thread::sleep(Duration::from_millis(500));

    // 青
    light_led(&mut pin1, 0)?;
    light_led(&mut pin2, 0)?;
    light_led(&mut pin3, 100)?;
    thread::sleep(Duration::from_millis(500));
    Ok(())
}
