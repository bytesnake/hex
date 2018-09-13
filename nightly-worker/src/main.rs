extern crate chrono;
extern crate hex_database;

use std::env;
use hex_database::{Collection, Event, events::Action};
use chrono::{TimeZone, Utc, Date, Duration};

fn main() {
    let db = env::args().skip(1).next()
        .map(|x| Collection::from_file(&x)).expect("Please specify database path");

    let newest_date = db.get_newest_summarise_day()
        .map(|x| Utc.datetime_from_str(&format!("{} 10:10:00", x), "%Y-%m-%d %H:%M:%S").unwrap().date())
        .unwrap_or(Utc::today().checked_sub_signed(Duration::days(2)).unwrap());

    let num_days = Utc::today().signed_duration_since(newest_date).num_days() - 1;

    if num_days == 0 {
        println!("Already done for yesterday!");
        return;
    }

    let mut days = vec![(0u32, 0u32, 0u32, 0u32); num_days as usize];

    let events: Vec<(Date<Utc>, Event)> = db.get_events().into_iter()
        .filter_map(|x| {
            Utc.datetime_from_str(&x.0, "%Y-%m-%d %H:%M:%S").map(|y| (y.date(), x.1)).ok()
        }).collect();

    for event in events {
        let diff = event.0.signed_duration_since(newest_date);

        if diff.num_days() < num_days {
            continue;
        }

        let idx = Utc::today().signed_duration_since(event.0).num_days() as usize - 1;

        match event.1.action() {
            Action::Connect(_) => days[idx].0 += 1,
            Action::PlaySong(_) => days[idx].1 += 1,
            Action::AddSong(_) => days[idx].2 += 1,
            Action::DeleteSong(_) => days[idx].3 += 1
        }
    }

    for i in 0..days.len() {
        let datestamp = Utc::today().checked_sub_signed(Duration::days(i as i64 + 1)).unwrap();
        let datestamp = datestamp.format("%Y-%m-%d");

        
        db.summarise_day(datestamp.to_string(), days[i].0, days[i].1, days[i].2, days[i].3).unwrap();
    }

    println!("{:#?}", days);

}
