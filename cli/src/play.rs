use std::thread;
use std::io::{self, Write, Read};
use std::fs::File;
use std::time::Duration;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, channel};
use crate::audio::AudioDevice;
use terminal_size::{Width, terminal_size};

use nix::sys::termios;

use hex_database::Track;
use hex_music_container::{Container, Configuration};

#[derive(Debug)]
pub enum Event {
    PauseContinue,
    Forward,
    Backward,
    Next,
    Prev,
    Quit
}

fn format_time(mut secs: f64) -> String {
    let mut out = String::new();
    let mut f = "s";

    if secs >= 60.0*60.0 {
        let hr = (secs / 60.0 / 60.0).floor();
        secs -= hr * 60.0 * 60.0;
        f = "h";

        out.push_str(&format!("{}:", hr));
    }
    if secs >= 60.0 {
        let min = (secs / 60.0).floor();
        secs -= min * 60.0;
        if f != "h" {
            f = "m";
        }

        out.push_str(&format!("{}:", min));
    }
    out.push_str(&format!("{}{}", secs.floor(), f));

    out
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
            match events.try_recv() {
                Ok(Event::Next) => {
                    continue 'outer;
                },
                Ok(Event::Prev) => {
                    idx -= 2;

                    continue 'outer;
                },
                Ok(Event::Quit) => return,
                _ => {}
            }

            println!("File {} not available", tracks[idx].key.to_string());
            thread::sleep(Duration::from_millis(500));
        }

        let file = File::open(data_path.join(tracks[idx].key.to_path())).unwrap();
        let mut container = Container::load(file).unwrap();

        println!("{} ({}) by {}", tracks[idx].title.clone().unwrap_or("Unknown".into()), tracks[idx].album.clone().unwrap_or("Unknown".into()), tracks[idx].composer.clone().unwrap_or("Unknown".into()));

        let mut pos = 0.0;
        let mut pause = false;
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
            print!("] {} / {}", format_time(pos), format_time(container.samples() as f64 / 48000.0));

            io::stdout().flush().unwrap();

            let mut written = 0;
            while written < buf.len() {
                match device.buffer(&buf, written) {
                    0 => {
                        thread::sleep(Duration::from_millis(50));
                    },
                    x => written += x
                }

                match events.try_recv() {
                    Ok(Event::Forward) => {
                        if pos + 10.0 < container.samples() as f64 / 48000.0 {
                            pos += 10.0;
                            container.seek_to_sample(pos as u32 * 48000);
                            device.clear();
                        }
                    },
                    
                    Ok(Event::Backward) => {
                        if pos - 10.0 >= 0.0 {
                            pos -= 10.0;
                            container.seek_to_sample(pos as u32 * 48000);
                            device.clear();
                        }
                    },
                    
                    Ok(Event::PauseContinue) => {
                        pause = !pause;
                    
                        if pause {
                            device.pause();
                        } else {
                            device.cont();
                        }
                    },
                    
                    Ok(Event::Next) => {
                        break 'inner;
                    },
                    Ok(Event::Prev) => {
                        if pos > 4.0 || idx == 0 {
                            idx -= 1;
                        } else {
                            idx -= 2;
                        }
                    
                        break 'inner;
                    },
                    Ok(Event::Quit) => {
                        device.shutdown();
                        return;
                    },
                    _ => {}
                }
            }
        }

        device.clear();

        idx += 1;

        println!(" Finished!\n");
    }

    device.shutdown();
}

pub fn play_tracks(data_path: PathBuf, tracks: Vec<Track>) {

    // setup terminal to pass arrows
    // Querying original as a separate, since `Termios` does not implement copy
    let orig_term = termios::tcgetattr(0).unwrap();
    let mut term = termios::tcgetattr(0).unwrap();
    // Unset canonical mode, so we get characters immediately
    term.local_flags.remove(termios::LocalFlags::ICANON);
    // Don't generate signals on Ctrl-C and friends
    term.local_flags.remove(termios::LocalFlags::ISIG);
    // Disable local echo
    term.local_flags.remove(termios::LocalFlags::ECHO);
    termios::tcsetattr(0, termios::SetArg::TCSADRAIN, &term).unwrap();
    let (sender, receiver) = channel();

    let handle = thread::spawn(move || player(data_path.to_path_buf(), tracks, receiver));

    for byte in io::stdin().bytes() {
        let res = match byte {
            Ok(32) => sender.send(Event::PauseContinue),
            Ok(65) => sender.send(Event::Forward),
            Ok(66) => sender.send(Event::Backward),
            Ok(68) => sender.send(Event::Prev),
            Ok(67) => sender.send(Event::Next),
            Ok(3) => {
                sender.send(Event::Quit).unwrap();

                handle.join().unwrap();
                break;
            },
            _ => Ok(())
        };

        //println!("{:?}", res);
        if let Err(_) = res {
            println!("ERROR");
            break;
        }
    }

    termios::tcsetattr(0, termios::SetArg::TCSADRAIN, &orig_term).unwrap();
}
