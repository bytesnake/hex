use std::path::PathBuf;
use hex_database::{Instance, Event, events::Action, GossipConf, objects::Fingerprint};
use chrono::{TimeZone, Utc, Date, Duration};

fn main() {
    let (conf, path) = match hex_conf::Conf::new() {
        Ok(x) => x,
        Err(err) => {
            eprintln!("Error: Could not load configuration {:?}", err);
            (hex_conf::Conf::default(), PathBuf::from("/opt/music/"))
        }
    };
    let db_path = path.join("music.db");

    let mut gossip = GossipConf::new();

    if let Some(ref peer) = conf.peer {
        gossip = gossip.id(peer.id());
    }

    let instance = Instance::from_file(&db_path, gossip);
    let (read, write) = (instance.reader(), instance.writer());

    let newest_date = read.get_latest_summary_day()
        .map(|x| Utc.datetime_from_str(&format!("{} 10:10:00", x), "%Y-%m-%d %H:%M:%S").unwrap().date())
        .unwrap_or(Utc::today().checked_sub_signed(Duration::days(2)).unwrap());

    let num_days = Utc::today().signed_duration_since(newest_date).num_days() - 1;

    if num_days == 0 {
        println!("Already done for yesterday!");
        return;
    }

    let mut days = vec![(0u32, 0u32); num_days as usize];

    let num_tracks = read.get_num_tracks();

    for day in 0..num_days {
        let num_transitions = read.get_num_transitions(day as u32);

        days[day as usize] = (num_tracks as u32, num_transitions as u32);
    }

    println!("{:?}", days);

    /*let tracks = read.get_tracks();
    let fps: Vec<Fingerprint> = tracks.iter().map(|x| x.fingerprint.clone()).collect();
    for i in 0..fps.len() {
        for j in 0..fps.len() {
            if i == j {
                continue;
            }
            if fps[i].iter().map(|x| *x as u64).sum::<u64>() == 0 {
                continue;
            }

            //println!("{}", cmp::min(fps[i].len(), fps[j].len()));
            let mut sum = 0.0;
            for k in 0..cmp::min(fps[i].len(), fps[j].len()) {
                let n = (fps[i][k] - fps[j][k]) as f64;
                sum += n*n;
            }

            if sum < 50000000.0 {
                println!("{:?} similar to {:?}", tracks[i].title, tracks[j].title);
                //println!("{:?}", fps[i]);
            }
        }
    }*/


    /*for i in 0..days.len() {
        let datestamp = Utc::today().checked_sub_signed(Duration::days(i as i64 + 1)).unwrap();
        let datestamp = datestamp.format("%Y-%m-%d");

        
        write.summarise_day(datestamp.to_string(), days[i].0, days[i].1).unwrap();
    }

    println!("{:#?}", days);*/
}
