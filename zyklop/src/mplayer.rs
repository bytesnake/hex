use std::io::{Read, Write, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Child, Stdio};
use std::time::Instant;
use hex2::{Result, StoreError, Playlist};

pub struct Mplayer {
    handle: Child,
    pos: usize,
    max_songs: usize,
    shuffle: bool,
}

impl Mplayer {
    pub fn from_list(files: &[PathBuf], shuffle: bool, position: Option<(usize, usize)>) -> Result<Self> {
        for song in files {
            if !song.exists() {
                let song_name = song.to_str().unwrap().to_string();
                return Err(StoreError::SongNotFound(song_name));
            }
        }

        if !Path::new("/usr/bin/mplayer").exists() {
            return Err(StoreError::BinaryMissing("mplayer".into()));
        }

        let mut main_handle = Command::new("/usr/bin/mplayer");
        let mut handle = if shuffle {
            main_handle.arg("-idle").arg("-shuffle")
        } else {
            main_handle.arg("-idle")
        };

        // convert pathbuf to strings and insert position if necessary
        let files = files.into_iter().map(|x| x.to_string_lossy().to_string()).collect::<Vec<_>>();

        /*if let Some(pos) = position {
            files.insert(
        };*/

        let mut handle = handle.arg("-quiet").args(&files)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // skip to the given song
        let mut pos = 0;
        if let (Some(handle), Some(p)) = (handle.stdin.as_mut(), position) {
            pos = p.0;
            for i in 0..p.0 {
                handle.write(b">")?;
            }
        }

        let stdout = handle.stdout.as_mut().unwrap();
        let stdout_reader = std::io::BufReader::with_capacity(8000 * 100, stdout);
        let stdout_lines = stdout_reader.lines();

        let mut correct = false;

        'outer: for line in stdout_lines.into_iter().filter_map(|x| x.ok()) {
            if line.contains("Starting playback...") {
                correct = true;
                break 'outer;
            } else if line.contains("Exiting...") {
                correct = false;
                break 'outer;
            }
        };

        if !correct {
            let output = handle.wait_with_output()?;
            let stderr = String::from_utf8_lossy(&output.stderr)
                .to_string();

            return Err(StoreError::MplayerFailed(stderr));
        } else {
            Ok(Mplayer { handle, pos, max_songs: files.len(), shuffle })
        }
    }

    pub fn from_url(url: &str) -> Result<Self> {
        if !Path::new("/usr/bin/mplayer").exists() {
            return Err(StoreError::BinaryMissing("mplayer".into()));
        }

        let mut handle = Command::new("/usr/bin/mplayer")
            .arg("-quiet")
            .arg(url)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdout = handle.stdout.as_mut().unwrap();
        let stdout_reader = std::io::BufReader::with_capacity(8000 * 10000, stdout);
        let stdout_lines = stdout_reader.lines();

        let mut correct = false;

        'outer: for line in stdout_lines.into_iter().filter_map(|x| x.ok()) {
            if line.contains("Starting playback...") {
                correct = true;
                break 'outer;
            } else if line.contains("Exiting...") {
                correct = false;
                break 'outer;
            }
        };

        if !correct {
            let output = handle.wait_with_output()?;
            let stderr = String::from_utf8_lossy(&output.stderr)
                .to_string();

            return Err(StoreError::MplayerFailed(stderr));
        } else {
            //drop(stdout_lines);
            //drop(stdout_reader);

            Ok(Mplayer { handle, pos: 0, max_songs: 0, shuffle: false })
        }
    }
    pub fn has_next(&self) -> bool {
        self.pos < self.max_songs-1
    }

    pub fn next(&mut self) -> Result<()> {
        self.pos += 1;
        self.handle.stdin.as_mut().unwrap().write(b">")?;

        dbg!(&self.pos);

        Ok(())
    }

    pub fn has_prev(&self) -> bool {
        self.pos > 0
    }

    pub fn prev(&mut self) -> Result<()> {
        self.pos -= 1;
        self.handle.stdin.as_mut().unwrap().write(b"<")?;
        dbg!(&self.pos);

        Ok(())
    }

    pub fn is_shuffled(&self) -> bool {
        self.shuffle
    }

    pub fn current_pos(&self) -> usize {
        self.pos
    }
}

impl Drop for Mplayer {
    fn drop(&mut self) {
        if let Err(err) = self.handle.kill() {
            eprintln!("could not stop mplayer process: {:?}", err);
        }
    }
}

