mod led;
mod events;

use rppal::system::DeviceInfo;
use anyhow::{Context, Result, anyhow};

use libmpv::Mpv;
use events::Event;

fn main() -> Result<()> {
    let device_info = DeviceInfo::new()
        .with_context(|| "Could not get Raspberry PI device info")?;

    println!("Blinking an LED on a {}.", device_info.model());

    let (led_state, led_thread) = led::spawn_led_thread()?;
    let (events_out, events_in) = events::spawn_events_thread();

    let mpv = Mpv::new()
        .map_err(|_| anyhow!("could not find MPV"))?;

    mpv.set_property("volume", 100)
        .map_err(|_| anyhow!("could not set volume for MPV"))?;

    mpv.set_property("vo", "null")
        .map_err(|_| anyhow!("could set vo MPV property"))?;

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
