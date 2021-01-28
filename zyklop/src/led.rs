use std::time::{Duration, Instant};
use std::thread::{spawn, JoinHandle};
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
pub use rppal::gpio::{Result, Gpio, OutputPin};

type Speed = f32;
type Pins = (OutputPin, OutputPin, OutputPin);

pub struct Color(pub u8, pub u8, pub u8, pub u8);

/// Set pins to specific color
pub fn set_color(pins: &mut Pins, color: &Color) -> Result<()> {
    let Color(ref r, ref g, ref b, ref a) = color;

    let r = 1. - (*r as f64) / 255. * (*a as f64) / 255.;
    let g = 1. - (*g as f64) / 255. * (*a as f64) / 255.;
    let b = 1. - (*b as f64) / 255. * (*a as f64) / 255.;

    pins.0.set_pwm_frequency(50., r)?;
    pins.1.set_pwm_frequency(50., g)?;
    pins.2.set_pwm_frequency(50., b)?;

    Ok(())
}

pub enum State {
    Sawtooth(Color, Speed),
    Sine(Color, Speed),
    Continuous(Color),
}

pub fn spawn_led_thread() -> Result<(Sender<State>, JoinHandle<()>)> {
    let mut pins = (
        Gpio::new()?.get(6)?.into_output(),
        Gpio::new()?.get(13)?.into_output(),
        Gpio::new()?.get(19)?.into_output(),
    );

    // set color to black, i.e. disable LEDs
    set_color(&mut pins, &Color(0, 0, 0, 0))?;

    // initialize by disabling LED
    let mut state = State::Continuous(Color(0, 0, 0, 0));

    // create channel
    let (sender, receiver) = channel();

    let res = spawn(move || {
        let now = Instant::now();

        loop {
            // read new state and update
            match receiver.recv_timeout(Duration::from_millis(10)) {
                Err(RecvTimeoutError::Disconnected) => break,
                Err(RecvTimeoutError::Timeout) => {},
                Ok(new_state) => {
                    if let State::Continuous(ref color) = new_state {
                        set_color(&mut pins, &color).unwrap()
                    }

                    state = new_state;
                }
            }

            // tick PWM routine
            if let State::Sawtooth(mut color, speed) = state {
                let cycle = now.elapsed().as_millis() as f32 / speed;
                let a = ((cycle * 255.) as u32 % 255) as u8;

                color.3 = a;
                set_color(&mut pins, &color).unwrap();

                state = State::Sawtooth(color, speed);
            }

            if let State::Sine(mut color, speed) = state {
                let cycle = now.elapsed().as_millis() as f32 / speed * std::f32::consts::PI;
                let a = ((cycle.sin() + 1.0) / 2.0 * 255.) as u8;

                color.3 = a;
                set_color(&mut pins, &color).unwrap();

                state = State::Sine(color, speed);
            }
        }
    });

    Ok((sender, res))
}
