use std::sync::mpsc::{Receiver, Sender, channel};

use spidev::{Spidev, SpidevOptions};
use rppal::gpio::{Gpio, Level};

use mfrc522::{MFRC522, pcd::Reg, picc::UID};

use std::thread;
use std::time::Duration;

const BUTTON_PINS: &[u64] = &[17, 27, 22];

#[derive(Debug)]
pub enum Event {
    ButtonPressed(u8),
    PowerButton(bool),
    NewCard(u32),
    CardLost
}

pub fn events() -> (Receiver<Vec<Event>>, Sender<u32>) {
    let (sender, recv) = channel();
    let (sender2, recv2) = channel();

    thread::spawn(|| events_fn(sender, recv2));

    (recv, sender2)
}

fn events_fn(sender: Sender<Vec<Event>>, recv: Receiver<u32>) {
    let gpio = Gpio::new().unwrap();
    let mut inputs: Vec<_> = BUTTON_PINS.iter().map(|pin| {
        gpio.get(*pin as u8).unwrap().into_input_pullup()
    }).collect();

    inputs.push(gpio.get(26).unwrap().into_input_pulldown());

    let mut pin = gpio.get(25).unwrap().into_output();
    pin.set_high();

    let mut spi = Spidev::open("/dev/spidev0.0").unwrap();
    let options = SpidevOptions::new()
        .lsb_first(false)
        .bits_per_word(8)
        .max_speed_hz(100_000)
        .mode(spidev::SPI_MODE_0)
        .build();

    spi.configure(&options).unwrap();

    let mut mfrc522 = MFRC522::init(&mut spi).expect("MFRC522 Initialization failed.");

    let mut card_avail = false;
    let mut uid = UID::default();
    let mut prev = vec![Level::High, Level::High, Level::High, Level::High];
    loop {
        let vers = mfrc522.register_read(Reg::Version).expect("Could not read version");

        //println!("VERSION: 0x{:x}", vers);
        if let Ok(new_id) = recv.try_recv() {
            if card_avail {
                let mut buffer = [0u8; 16];
                buffer[0] = (new_id << 24) as u8;
                buffer[1] = (new_id << 16) as u8;
                buffer[2] = (new_id << 8) as u8;
                buffer[3] = (new_id << 0) as u8;

                if mfrc522.mifare_write(8, &buffer).is_ok() {
                    println!("Id written ..");
                } else {
                    panic!("Could not write ID :(!");
                }

                // reread the id ..
                card_avail = false;
            }
        }

        let vals: Vec<Level> = inputs.iter().map(|dev| dev.read()).collect();

        let mut events = Vec::new();
        //if let Ok(vals) = vals {
            if vals[0] == Level::Low && prev[0] == Level::High {
                events.push(Event::ButtonPressed(0));
            }
            if vals[1] == Level::Low && prev[1] == Level::High {
                events.push(Event::ButtonPressed(1));
            }
            if vals[2] == Level::Low && prev[2] == Level::High {
                events.push(Event::ButtonPressed(2));
            }
            if vals[3] == Level::Low && prev[3] == Level::High {
                events.push(Event::PowerButton(false));
            }
            if vals[3] == Level::High && prev[3] == Level::Low {
                events.push(Event::PowerButton(true));
            }

            prev = vals;
        //}

        if card_avail {
            let mut buffer = [0_u8; 18];
            let (read_status, nread) = mfrc522.mifare_read(4, &mut buffer);
            if !read_status.is_ok() || nread == 0 {
                println!("Lost: {:?}", read_status);
                card_avail = false;
                events.push(Event::CardLost);
            }
        } else {
            let new_card = mfrc522.picc_is_new_card_present();
            //println!("{:?}", new_card);

            if let Some(_) = new_card {
                uid.clear();
                println!("New card detected!");
                //println!("New card detected. ATQA: {:04x}", atqa.bits());
                let status = mfrc522.picc_select(&mut uid);
            
                if status.is_ok() {
                    let mut buffer = vec![0_u8; 18];
                    let (read_status, nread) = mfrc522.mifare_read(8, &mut buffer);
                    if read_status.is_ok() && nread > 0 {
                        card_avail = true;
                        let id = ((buffer[0] as u32) << 24) |
                                 ((buffer[1] as u32) << 16) |
                                 ((buffer[2] as u32) << 8)  |
                                 ((buffer[3] as u32) << 0);

                        events.push(Event::NewCard(id));
                    }
                }

            }
        }

        if events.len() > 0 {
            println!("{:?}", events);
            sender.send(events).unwrap();
        }

        thread::sleep(Duration::from_millis(40));
    }
}
