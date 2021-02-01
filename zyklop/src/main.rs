mod led;
mod events;
mod mplayer;

use std::sync::mpsc::{Sender, RecvTimeoutError};
use std::time::Duration;
use std::thread;

use rppal::system::DeviceInfo;
use anyhow::{Context, Result, anyhow};

use hex2::StoreError;

use events::Event;
use mplayer::Mplayer;
use led::Color;

pub enum State {
    Idle,
    Playing(u32, Mplayer),
    Programming(u32, u32),
}

fn process_error(state: State, led_state: &Sender<led::State>, duration: u32, error: anyhow::Error) -> Result<State> {
    eprintln!("Got error: {:?}", error);

    led_state.send(led::State::Sine(led::Color(255, 0, 0, 255), 1000.0))?;

    thread::sleep(Duration::from_millis(duration as u64));

    match &state {
        State::Playing(_, _) => led_state.send(led::State::Continuous(led::Color(0, 0, 255, 255)))?,
        State::Programming(_, _) => led_state.send(led::State::Continuous(led::Color(0, 255, 0, 255)))?,
        State::Idle => led_state.send(led::State::Continuous(led::Color(255, 255, 255, 255)))?,
    }

    Ok(state)
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

    'outer: loop {
        // get a new input event and convert into reference to array
        let answ = match events_out.recv_timeout(Duration::from_millis(50)) {
            Ok(new_input) => new_input,
            Err(RecvTimeoutError::Timeout) => continue 'outer,
            Err(RecvTimeoutError::Disconnected) => 
                break 'outer Err(anyhow!("pipe breaked!")),
        };

        let new_state = match (answ, state) {
            (Event::NewCard(id), state @ State::Idle) => {
                if let Ok(pl) = store.playlist_by_card(id) {
                    match Mplayer::from_list(vec![]) {
                        Ok(mplayer) => Ok(State::Playing(id, mplayer)),
                        Err(err) => process_error(state, led_state, 2000, err.into()),
                    }
                } else {
                    process_error(state, led_state, 2000,
                        StoreError::PlaylistNotFound(id.to_string()).into())
                }
            },
            (Event::CardLost, _) => Ok(State::Idle),
            (Event::ButtonPressed(0), State::Playing(id, mut player)) => {
                player.next()?;

                Ok(State::Playing(id, player))
            },
            (Event::ButtonPressed(1), State::Playing(id, mut player)) => {
                player.prev()?;

                Ok(State::Playing(id, player))
            },
            (_, state) => Ok(state)
        };

        match &new_state {
            Ok(State::Idle) => led_state.send(led::State::Continuous(Color(255, 255, 255, 255)))?,
            Ok(State::Playing(_, _)) => led_state.send(led::State::Continuous(Color(0, 0, 255, 255)))?,
            Ok(State::Programming(_, _)) => led_state.send(led::State::Continuous(Color(0, 255, 0, 255)))?,
            _ => {}
        }

        state = new_state?;
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
