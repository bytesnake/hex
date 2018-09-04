extern crate sysfs_gpio;

use sysfs_gpio::{Direction, Pin};
use std::thread;
use std::time::Duration;

const BUTTON_PINS: &[u64] = &[1016, 1014, 1018, 1019];

#[derive(Debug)]
enum Event {
    ButtonPressed(u8)
}

fn main() {
    let inputs: Result<Vec<_>, _> = BUTTON_PINS.iter().map(|pin| {
        let input = Pin::new(*pin);
        
        input.export()
            .and_then(|x| input.set_direction(Direction::In))
            .map(|_| input)
    }).collect();

    let inputs = inputs.unwrap();

    let mut prev = vec![1,1,1,1];
    loop {
        let vals: Result<Vec<u8>, _> = inputs.iter().map(|dev| dev.get_value()).collect();

        let mut events = Vec::new();
        if let Ok(vals) = vals {
            if vals[0] < prev[0] {
                events.push(Event::ButtonPressed(0));
            }
            if vals[1] < prev[1] {
                events.push(Event::ButtonPressed(1));
            }
            if vals[2] < prev[2] {
                events.push(Event::ButtonPressed(2));
            }
            if vals[3] < prev[3] {
                events.push(Event::ButtonPressed(3));
            }

            if events.len() > 0 {
                println!("{:?}", events);
            }

            prev = vals;
        }

        thread::sleep(Duration::from_millis(50));
    }
}
