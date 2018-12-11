use std::thread;
use std::io::{self, Write, Read};
use std::fs::File;
use std::time::Duration;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, channel};
use audio::AudioDevice;
use terminal_size::{Width, terminal_size};

use nix::sys::termios;
use futures::Future;

use hex_database::{Track, TrackKey, Instance};
use hex_music_container::{Container, Configuration};

#[derive(Debug)]
pub enum Event {
    Next,
    Prev
}

pub fn player(data_path: PathBuf, tracks: Vec<Track>, events: Receiver<Event>) {
    let mut device = AudioDevice::new();
    let width = match terminal_size() {
        Some((Width(w),_)) => w,
        _ => 64
    };

    let mut idx = 0;
    'outer: loop {
        if idx == tracks.len() {
            break;
        }

        // wait till file available
        while !data_path.join(tracks[idx].key.to_path()).exists() {
            println!("File {} not available", tracks[idx].key.to_string());
            thread::sleep(Duration::from_millis(500));
        }

        let file = File::open(data_path.join(tracks[idx].key.to_path())).unwrap();
        let mut container = Container::load(file).unwrap();

        println!("{}", tracks[idx].title.clone().unwrap_or("Unknown".into()));

        let mut pos = 0.0;
        'inner: while let Ok(buf) = container.next_packet(Configuration::Stereo) {
            pos += buf.len() as f64 / 48000.0 / 2.0;

            print!("\rPlaying [");
            for i in 0..(width - 30) as usize {
                if i < ((width - 30) as f64 * (pos / (container.samples() as f64 / 48000.0))) as usize {
                    print!("#");
                } else {
                    print!(" ");
                }
            }
            print!("]");

            io::stdout().flush().unwrap();

            device.buffer(&buf);

            match events.try_recv() {
                Ok(Event::Next) => {
                    break 'inner;
                },
                Ok(Event::Prev) => {
                    if pos > 4.0 {
                        idx -= 1;
                    } else {
                        idx -= 2;
                    }

                    break 'inner;
                },
                _ => {}
            }
        }

        device.clear();

        idx += 1;

        println!(" Finished!\n");
    }

    device.shutdown();
}

pub fn play_tracks(data_path: PathBuf, tracks: Vec<Track>, mut instance: Instance) {

    // setup terminal to pass arrows
    // Querying original as a separate, since `Termios` does not implement copy
    let orig_term = termios::tcgetattr(0).unwrap();
    let mut term = termios::tcgetattr(0).unwrap();
    // Unset canonical mode, so we get characters immediately
    term.local_flags.remove(termios::LocalFlags::ICANON);
    // Don't generate signals on Ctrl-C and friends
    //term.local_flags.remove(termios::LocalFlags::ISIG);
    // Disable local echo
    term.local_flags.remove(termios::LocalFlags::ECHO);
    termios::tcsetattr(0, termios::SetArg::TCSADRAIN, &term).unwrap();

    let (sender, receiver) = channel();

    let keys: Vec<TrackKey> = tracks.iter().map(|x| x.key.clone()).collect();
    let path_copy = data_path.clone();
    thread::spawn(move || {
        for key in keys {
            if !path_copy.join(key.to_string()).exists() {
                instance.ask_for_file(key.to_vec()).wait().unwrap();
            }
        }
    });

    let handle = thread::spawn(move || player(data_path.to_path_buf(), tracks, receiver));

    for byte in io::stdin().bytes() {
        match byte {
            Ok(68) => sender.send(Event::Prev).unwrap(),
            Ok(67) => sender.send(Event::Next).unwrap(),
            _ => {}
        }
    }

    termios::tcsetattr(0, termios::SetArg::TCSADRAIN, &orig_term).unwrap();
}
