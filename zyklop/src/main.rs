mod led;
mod events;

use rppal::system::DeviceInfo;
use rppal::gpio::Result;

use events::Event;

fn main() -> Result<()> {
    println!("Blinking an LED on a {}.", DeviceInfo::new().unwrap().model());

    let (led_state, led_thread) = led::spawn_led_thread()?;
    let (events_out, events_in) = events::events();

    //led_state.send(
        //led::State::Sine(led::Color(0, 255, 237, 0), 1000.0)).unwrap();

    loop {
        let answ = events_out.recv().unwrap();

        match answ[0] {
            Event::NewCard(_) => {
                led_state.send(
                    led::State::Sine(led::Color(0, 255, 237, 0), 1000.0)).unwrap();
            },
            Event::CardLost => {
                led_state.send(
                    led::State::Continuous(led::Color(0, 0, 0, 0))).unwrap();
            },
            _ => {}
        }

        dbg!(&answ);
    }

    //led_thread.join().unwrap();

    Ok(())
}
