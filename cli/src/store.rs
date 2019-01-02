use std::slice;
use std::io::Read;
use std::fs::File;
use std::process::Command;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use id3::Tag;
use chromaprint::Chromaprint;

use hex_database::Track;
use hex_music_container::{Configuration, Container};
use hex_database::View;

pub fn store(view: &View, path: &Path, data_path: PathBuf) {
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

        // calculate fingerprint
        let mut ctx = Chromaprint::new();
        ctx.start(48000, 2); 
        ctx.feed(&data);
        ctx.finish();

        let mut track = Track::empty(
            ctx.raw_fingerprint().unwrap().into_iter().map(|x| x as u32).collect(),
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

        view.add_track(track).unwrap();
    }
}
