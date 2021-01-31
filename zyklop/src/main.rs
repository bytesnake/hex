mod led;
mod events;
mod mplayer;
mod state;

use std::sync::mpsc::Sender;
use std::time::Duration;

use rppal::system::DeviceInfo;
use anyhow::{Context, Result, anyhow};

use hex2::StoreError;

use events::Event;
use mplayer::Mplayer;

pub enum State {
    Idle,
    Playing(u32, Mplayer),
    Programming(u32, u32),
    Error(u32, anyhow::Error, State),
}

fn process(led_state: &Sender<led::State>) -> Result<()> {
    let device_info = DeviceInfo::new()
        .with_context(|| "Could not get Raspberry PI device info")?;

    println!("Starting Zyklop on device {}", device_info.model());

    // spawn LED ring and events thread
    let (events_out, events_in) = events::spawn_events_thread();

    // open music storage
    let path = std::env::var("ZYKLOP_PATH")
        .map_err(|_| anyhow!("could not find path in `ZYKLOP_PATH`"))?;

    let mut store = hex2::Store::from_path(&path)?;
    let mut state = State::Idle;

    loop {
        // get a new input event
        let answ = events_out.recv_timeout(Duration::from_millis(50));
        let answ_ref = answ.as_ref().map(|x| &x[..]);

        state = match (answ_ref, state) {
            (Ok(&[Event::NewCard(id)]), State::Idle) => {
                //led_state.send(
                //led::State::Sine(led::Color(0, 255, 237, 0), 1000.0)).unwrap();

                if let Ok(pl) = store.playlist_by_card(id) {
                    match Mplayer::from_list(vec![]) {
                        Ok(mplayer) => State::Playing(id, mplayer),
                        Err(err) => State::Error(2000, err.into(), State::Idle)
                    }
                } else {
                    State::Error(2000, StoreError::PlaylistNotFound(id.to_string()).into(), Satate::Idle)
                }
            },
            (Ok(&[Event::CardLost]), State::Playing(_, _)) => State::Idle,
            (Err(_), State::Error(duration, reason, next_state)) => {
                eprintln!("Got error: {:?}", reason);

                led_state.send(led::State::Sine(led::Color(255, 0, 0, 255), 1000.0))?;

                thread::sleep(Duration::from_millis(duration));

                led_state
            },
            (_, state) => state
        };

        dbg!(&answ);
    }
}

fn main() -> Result<()> {
    let (led_state, led_thread) = led::spawn_led_thread()?;
    let res = process(&led_state);

    // after process 
    if res.is_ok() {
        led_state.send(led::State::Continuous(led::Color(255, 255, 255, 255)))?;
    } else {
        led_state.send(led::State::Sine(led::Color(255, 0, 0, 255), 2000.0))?;
    }

    res
}
