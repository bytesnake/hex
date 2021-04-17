mod led;
mod events;
mod mplayer;

use std::sync::mpsc::{Sender, RecvTimeoutError};
use std::time::Duration;
use std::thread;

use rppal::system::DeviceInfo;
use anyhow::{Context, Result, anyhow};

use hex2::{Playlist, StoreError};

use events::Event;
use mplayer::Mplayer;
use led::Color;

pub enum State {
    Idle,
    Playing(Playlist, Mplayer),
    Programming(u32, Mplayer, Vec<Playlist>),
}

fn process_error(state: State, led_state: &Sender<led::State>, duration: u32, error: anyhow::Error) -> Result<State> {
    eprintln!("Got error: {:?}", error);

    led_state.send(led::State::Sine(led::Color(255, 0, 0, 255), 300.0))?;

    thread::sleep(Duration::from_millis(duration as u64));

    match &state {
        State::Playing(pl, _) => {
            if pl.radio_url.is_some() {
                led_state.send(led::State::Continuous(led::Color(0, 255, 255, 255)))?
            } else {
                led_state.send(led::State::Continuous(led::Color(0, 0, 255, 255)))?
            }
        },
        State::Programming(..) => led_state.send(led::State::Continuous(led::Color(255, 255, 0, 255)))?,
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
                let playlist = store.playlists().into_iter()
                    .filter(|x| x.card_id.map(|x| x == id).unwrap_or(false))
                    .next();

                if let Some(pl) = playlist {
                    if pl.radio_url.is_some() {
                        led_state.send(led::State::Sine(led::Color(0, 255, 255, 255), 500.0))?
                    } else {
                        led_state.send(led::State::Sine(led::Color(0, 0, 255, 255), 500.0))?
                    }

                    let res = match &pl.radio_url {
                        Some(url) => Mplayer::from_url(url),
                        None => Mplayer::from_list(&pl.files, false, pl.position)
                    };

                    match res {
                        Ok(mplayer) => {
                            Ok(State::Playing(pl.clone(), mplayer))
                        },
                        Err(err) => process_error(state, led_state, 1000, err.into()),
                    }
                } else {
                    // get new card id
                    let new_card_id = store.next_card_id();
                    // get all playlists without a corresponding cards
                    let playlists = store.playlists_without_card();
                    // find first song for each playlist
                    let sample_files = playlists
                        .iter().filter_map(|x| {
                            store.get_files(&x.name).into_iter().next().clone()
                        }).collect::<Vec<_>>();

                    // create player from those songs and change state
                    match Mplayer::from_list(&sample_files, false, None) {
                        Ok(mplayer) => {
                            Ok(State::Programming(new_card_id, mplayer, playlists))
                        },
                        Err(err) => process_error(state, led_state, 1000, err.into()),
                    }
                }
            },
            (Event::CardLost, state) => {
                if let State::Playing(mut pl, mplayer) = state {
                    store.set_position(&pl.name, mplayer.current_pos())?;
                }

                Ok(State::Idle)
            },
            (Event::ButtonPressed(2), State::Playing(id, mut player)) => {
                if player.has_next() {
                    player.next()?;

                    Ok(State::Playing(id, player))
                } else {
                    process_error(State::Playing(id, player), led_state, 2000, StoreError::ReachedEndOfPlaylist.into())
                }
            },
            (Event::ButtonPressed(0), State::Playing(id, mut player)) => {
                if player.has_prev() {
                    player.prev()?;

                    Ok(State::Playing(id, player))
                } else {
                    process_error(State::Playing(id, player), led_state, 2000, StoreError::ReachedBeginningOfPlaylist.into())
                }
            },
            (Event::ButtonPressed(1), State::Playing(pl, player)) => {
                if pl.allow_random {
                    let was_shuffled = player.is_shuffled();
                    drop(player);

                    match Mplayer::from_list(&store.get_files(&pl.name), !was_shuffled, None) {
                        Ok(mplayer) => {
                            Ok(State::Playing(pl, mplayer))
                        },
                        Err(err) => process_error(State::Idle, led_state, 1000, err.into()),
                    }
                } else {
                    process_error(State::Playing(pl, player), led_state, 2000, StoreError::RandomNotAllowed.into())
                }
            },
            (Event::ButtonPressed(2), State::Programming(card_id, mut player, pls)) => {
                if player.has_next() {
                    player.next()?;

                    Ok(State::Programming(card_id, player, pls))
                } else {
                    process_error(State::Programming(card_id, player, pls), led_state, 2000, StoreError::ReachedEndOfPlaylist.into())
                }
            },
            (Event::ButtonPressed(0), State::Programming(card_id, mut player, pls)) => {
                if player.has_prev() {
                    player.prev()?;

                    Ok(State::Programming(card_id, player, pls))
                } else {
                    process_error(State::Programming(card_id, player, pls), led_state, 2000, StoreError::ReachedBeginningOfPlaylist.into())
                }
            },
            (Event::ButtonPressed(1), State::Programming(card_id, player, pls)) => {
                let playlist = pls[player.current_pos()].clone();
                store.set_playlist_card_id(&playlist.name, card_id)?;
                events_in.send(card_id)?;
                store.save()?;

                match Mplayer::from_list(&playlist.files, false, None) {
                    Ok(mplayer) => {
                        Ok(State::Playing(playlist, mplayer))
                    },
                    Err(err) => process_error(State::Idle, led_state, 1000, err.into()),
                }
            },
            (_, state) => Ok(state)
        };

        // update the LED to the new state
        match &new_state {
            Ok(State::Idle) => led_state.send(led::State::Continuous(Color(255, 255, 255, 255)))?,
            Ok(State::Playing(pl, _)) => {
                if pl.radio_url.is_some() {
                    led_state.send(led::State::Continuous(led::Color(0, 255, 255, 255)))?
                } else {
                    led_state.send(led::State::Continuous(led::Color(0, 0, 255, 255)))?
                }
            },
            Ok(State::Programming(..)) => led_state.send(led::State::Continuous(Color(255, 255, 0, 255)))?,
            _ => {}
        }

        state = new_state?;
    }
}

fn main() -> Result<()> {
    let (led_state, led_thread) = led::spawn_led_thread()?;
    let res = process(&led_state);

    // we bailed out either because of an error or because we are shutting down the Zyklop
    if res.is_ok() {
        led_state.send(led::State::Sine(led::Color(255, 255, 255, 255), 2000.0))?;
    } else {
        led_state.send(led::State::Continuous(led::Color(255, 0, 0, 255)))?;
        eprintln!("{:?}", res);
    }

    led_thread.join().unwrap();

    res
}
