use std::slice;
use std::io::Read;
use std::fs::File;
use std::process::Command;
use std::path::Path;
use walkdir::WalkDir;
use id3::Tag;

use hex_database::{Track, Writer};
use hex_music_container::{Configuration, Container};

pub fn store(write: &Writer, path: &Path, data_path: &Path) {
    let mut files = Vec::new();
    for e in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        if e.metadata().unwrap().is_file() {
            let path = e.path();
            let extension = path.extension().unwrap();
            if extension == "aac" || extension == "mp3" || extension == "wav" || extension == "ogg" {
                files.push(path.to_path_buf());
            }
        }
    }

    for file in files {
        println!("Converting file {:?}", file.to_str());
        let tag = Tag::read_from_path(&file);

        // convert to pcm file 
        let mut cmd = Command::new("ffmpeg")
            .arg("-y")
            .arg("-hide_banner")
            //.arg("-loglevel").arg("panic")
            .arg("-i").arg(&file)
            .arg("-ar").arg("48000")
            .arg("-ac").arg("2")
            .arg("-f").arg("s16le")
            .arg("/tmp/hex-cli-audio")
            .spawn()
            .expect("Failed to spawn ffmpeg!");

        cmd.wait().unwrap();

        let mut audio_file = File::open("/tmp/hex-cli-audio").unwrap();
        let mut data = Vec::new();

        audio_file.read_to_end(&mut data).unwrap();

        let data: &[i16] = unsafe {
            slice::from_raw_parts(
                data.as_ptr() as *const i16,
                data.len() / 2
            )
        };

        println!("Finished converting with {} samples", data.len());

        let fingerprint = hex_database::utils::get_fingerprint(2, &data).unwrap();

        let mut track = Track::empty(
            fingerprint,
            data.len() as f64 / 48000.0 / 2.0
        );

        if let Ok(tag) = tag {
            if let Some(title) = tag.title().map(|x| x.to_string()) {
                track.title = Some(title);
            } else {
                track.title = Some(file.file_stem().unwrap().to_str().unwrap().into());
            }

            track.album = tag.album().map(|x| x.to_string());
            track.interpret = tag.artist().map(|x| x.to_string());
            track.composer = tag.artist().map(|x| x.to_string());
                                   
        } else {
            track.title = Some(file.file_stem().unwrap().to_str().unwrap().into());
        }

        // store with music container
        let file = File::create(data_path.join(track.key.to_path())).unwrap();
        Container::save_pcm(Configuration::Stereo, data.to_vec(), file, None).unwrap();

        println!("Add track with key {}", track.key.to_string());

        write.add_track(track).unwrap();
    }
}
