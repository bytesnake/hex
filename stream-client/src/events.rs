use std::sync::mpsc::{Receiver, Sender, channel};

use spidev::{self, Spidev, SpidevOptions};
use sysfs_gpio::{Pin, Direction};

use mfrc522::{MFRC522, pcd::Reg, picc::UID};

use std::thread;
use std::time::Duration;

const BUTTON_PINS: &[u64] = &[1016, 1014, 1018, 1019];

#[derive(Debug)]
pub enum Event {
    ButtonPressed(u8),
    NewCard(Vec<u8>),
    CardLost
}

pub fn events() -> Receiver<Vec<Event>> {
    let (sender, recv) = channel();

    thread::spawn(|| events_fn(sender));

    recv
}

fn events_fn(sender: Sender<Vec<Event>>) {
    let inputs: Result<Vec<_>, _> = BUTTON_PINS.iter().map(|pin| {
        let input = Pin::new(*pin);
        
        input.export()
            .and_then(|_| input.set_direction(Direction::In))
            .map(|_| input)
    }).collect();

    let inputs = inputs.unwrap();

    let mut spi = Spidev::open("/dev/spidev32766.0").unwrap();
    let options = SpidevOptions::new()
        .lsb_first(false)
        .bits_per_word(8)
        .max_speed_hz(500_000)
        .mode(spidev::SPI_MODE_0)
        .build();

    spi.configure(&options).unwrap();

    let pin = Pin::new(1013);
    pin.export().unwrap();
    while !pin.is_exported() {}
    pin.set_direction(Direction::Out).unwrap();

    pin.set_value(1).unwrap();

    let mut mfrc522 = MFRC522::init(&mut spi).expect("MFRC522 Initialization failed.");

    let vers = mfrc522.register_read(Reg::Version).expect("Could not read version");

    println!("VERSION: 0x{:x}", vers);

    let mut card_avail = false;
    let mut uid = UID::default();
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

            prev = vals;
        }

        if card_avail {
            let mut buffer = [0_u8; 18];
            let (read_status, nread) = mfrc522.mifare_read(8, &mut buffer);
            if !read_status.is_ok() || nread == 0 {
                card_avail = false;
                events.push(Event::CardLost);
            }
        } else {
            let new_card = mfrc522.picc_is_new_card_present();

            if let Some(_) = new_card {
                //println!("New card detected. ATQA: {:04x}", atqa.bits());
                let status = mfrc522.picc_select(&mut uid);
            
                if status.is_ok() {
                    let mut buffer = vec![0_u8; 18];
                    let (read_status, nread) = mfrc522.mifare_read(8, &mut buffer);
                    if read_status.is_ok() && nread > 0 {
                        card_avail = true;
                        events.push(Event::NewCard(buffer));
                    }
                }

                uid.clear();
            }
        }

        if events.len() > 0 {
            sender.send(events).unwrap();
            //println!("{:?}", events);
        }

        thread::sleep(Duration::from_millis(100));
    }
}
