use std::io::Write;
use std::path::Path;
use std::slice;
use std::thread;
use std::fs::{self, File};
use std::process::Command;
use std::rc::Rc;
use std::cell::RefCell;
use std::borrow::{BorrowMut, Borrow};

use futures::{IntoFuture, Future, Stream};
use futures::sync::mpsc::{channel, Sender, Receiver};
use tokio_core::reactor::Handle;

use error::{Result, Error};

use hex_music_container::{Container, Configuration, error::Error as MusicError};
use hex_database::Track;

fn worker(mut sender: Sender<DownloadProgress>, id: String, format: String, tracks: Vec<Track>, num_channel: u32, data_path: String) -> Result<()> {
    let mut out_files = Vec::new();
    let download_path = Path::new(&data_path).join("download");
    println!("start working!");

    for i in 0..tracks.len() {
        println!("processing {}", i);
        sender.try_send(DownloadProgress { 
            id: id.clone(),
            format: format.clone(),
            progress: i as f32 / tracks.len() as f32,
            download: None
        }).map_err(|_| Error::ChannelFailed)?;

        let file_path = Path::new(&data_path).join(&tracks[i].key);

        let file_path_out = download_path
            .join(&tracks[i].interpret.clone().unwrap_or("unknown".into()))
            .join(&tracks[i].album.clone().unwrap_or("unknown".into()));

        fs::create_dir_all(&file_path_out).unwrap();

        let file_path_out = file_path_out.join(tracks[i].title.clone().unwrap_or(tracks[i].key.clone()));

        let file = File::open(&file_path)
            .map_err(|err| Error::Io(err))?;

        let mut container = Container::load(file)
            .map_err(|err| Error::MusicContainer(err))?;

        let mut out = File::create(&file_path_out)
            .map_err(|err| Error::Io(err))?;

        println!("convert start");
        loop {
            match container.next_packet(Configuration::Stereo) {
                Ok(buf) => { 
                    let buf: &[u8] = unsafe {
                        slice::from_raw_parts(
                            buf.as_ptr() as *const u8,
                            buf.len() * 2
                        )
                    };

                    out.write(&buf); 
                },
                Err(MusicError::ReachedEnd) => break,
                Err(err) => { return Err(Error::MusicContainer(err)); }
            }
        }
        println!("convert end");

        let converted_file = file_path_out.with_extension(format.clone());

        Command::new("ffmpeg")
            .arg("-y")
            .arg("-ar").arg("48k")
            .arg("-ac").arg("2")
            .arg("-f").arg("s16le")
            .arg("-i").arg(file_path_out.to_str().unwrap())
            .arg(converted_file.to_str().unwrap())
            .spawn().expect("Could not start ffmpeg!").wait().unwrap();

        println!("ffmpeg end");
        out_files.push(converted_file);

    }

    Command::new("tar")
        .arg("cvzf")
        .arg(download_path.join(format!("{}.tar.gz", id)))
        .args(out_files)
        .spawn().expect("Could not start tar!").wait().unwrap();

    sender.try_send(DownloadProgress { 
        id: id.clone(),
        format: format,
        progress: 1.0,
        download: Some(format!("/data/download/{}.tar.gz", id))
    }).map_err(|_| Error::ChannelFailed)?;

    Ok(())
}

#[derive(Serialize, Debug, Clone)]
pub struct DownloadProgress {
    id: String,
    format: String,
    progress: f32,
    download: Option<String>
}

impl DownloadProgress {
    pub fn empty() -> DownloadProgress {
        DownloadProgress {
            id: "".into(),
            format: "".into(),
            progress: 0.0,
            download: None
        }
    }
}

pub struct DownloadState {
    pub handle: Handle,
    thread: thread::JoinHandle<Result<()>>,
    progress: Rc<RefCell<DownloadProgress>>
}

impl DownloadState {
    pub fn new(handle: Handle, id: String, format: String, tracks: Vec<Track>, num_channel: u32, data_path: String) -> DownloadState {
        let (sender, recv) = channel(10);

        let thread = thread::spawn(move || {
            let res = worker(sender, id, format, tracks, num_channel, data_path)?;

            Ok(())
        });

        let progress = Rc::new(RefCell::new(DownloadProgress::empty()));

        let progress2 = progress.clone();
        let hnd = recv.map(move |x| {
            *((*progress2).borrow_mut()) = x;

            ()
        }).for_each(|_| Ok(())).into_future().map(|_| ()).map_err(|_| ());

        handle.spawn(hnd);

        DownloadState {
            handle: handle,
            thread: thread,
            progress: progress
            
        }
    }

    pub fn progress(&self) -> DownloadProgress {
        let tmp: DownloadProgress = (*self.progress).borrow().clone();

        tmp
    }
}
