use std::mem;
use std::sync::mpsc::{Receiver, Sender, channel};

use spidev::{self, Spidev, SpidevOptions};
use sysfs_gpio::{Pin, Direction};

use mfrc522::{picc, MFRC522, pcd::Reg, picc::UID, picc::mifare};

use std::thread;
use std::time::Duration;

const BUTTON_PINS: &[u64] = &[1016, 1014, 1018, 1019];

#[derive(Debug)]
pub enum Event {
    ButtonPressed(u8),
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
                uid.clear();
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
            sender.send(events).unwrap();
            //println!("{:?}", events);
        }

        thread::sleep(Duration::from_millis(100));
    }
}
