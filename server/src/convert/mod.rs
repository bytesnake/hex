pub mod ffmpeg;
pub mod youtube;
pub mod opus;
pub mod download;

use std::mem;
use std::path::Path;
use std::rc::Rc;
use std::cell::RefCell;

use tokio_core::reactor::Handle;
use futures::{Stream, Future, IntoFuture};

use hex_database::Track;

pub use self::download::{DownloadState, DownloadProgress};

#[derive(Serialize, Debug)]
pub struct UploadProgress {
    pub desc: String,
    pub kind: String,
    pub progress: f32,
    pub key: String,
    pub track_key: Option<String>
}

pub enum UploadState {
    YoutubeDownload {
        downloader: youtube::Downloader,
        state: Rc<RefCell<youtube::State>>,
        key: String
    },
    ConvertingFFMPEG {
        converter: ffmpeg::Converter,
        state: Rc<RefCell<ffmpeg::State>>,
        key: String
    },
    ConvertingOpus {
        converter: opus::Converter,
        state: Rc<RefCell<opus::State>>,
        key: String,
    },
    Finished(Option<(String, String, String)>)
}

impl UploadState {
    pub fn youtube(key: String, path: &str, handle: Handle) -> UploadState {
        let mut dwnd = youtube::Downloader::new(handle.clone(), path);

        let state = Rc::new(RefCell::new(youtube::State::empty()));
        let state2 = state.clone();

        let hnd = dwnd.state().map(move |x| {
            *(*state2).borrow_mut() = x;

            ()
        });

        dwnd.spawn(hnd);
        handle.spawn(dwnd.child().into_future().map(|_| ()).map_err(|_| ()));

        UploadState::YoutubeDownload {
            downloader: dwnd,
            state: state,
            key: key
        }
    }

    pub fn converting_ffmpeg(handle: Handle, desc: String, key: String, data: &[u8], format: &str) -> UploadState {
        let mut dwnd = ffmpeg::Converter::new(handle.clone(), desc.clone(), data, format).unwrap();

        let state = Rc::new(RefCell::new(ffmpeg::State::empty(desc, "", "")));
        let state2 = state.clone();

        let hnd = dwnd.state().map(move |x| {
            *(*state2).borrow_mut() = x;

            ()
        });

        dwnd.spawn(hnd);
        handle.spawn(dwnd.child().into_future().map(|_| ()).map_err(|_| ()));

        UploadState::ConvertingFFMPEG {
            converter: dwnd,
            state: state,
            key: key
        }
    }

    pub fn converting_opus(handle: Handle, key: String, desc: String, samples: &[i16], duration: f32, num_channel: u32, data_path: String) -> UploadState {
        let mut dwnd = opus::Converter::new(handle.clone(), desc.clone(), Vec::from(samples), duration, num_channel, data_path);

        let state = Rc::new(RefCell::new(opus::State::empty(desc)));
        let state2 = state.clone();

        let hnd = dwnd.state().map(move |x| {
            *(*state2).borrow_mut() = x;

            ()
        });

        dwnd.spawn(hnd);
        //handle.spawn(dwnd.child().into_future().map(|_| ()).map_err(|_| ()));

        UploadState::ConvertingOpus {
            converter: dwnd,
            state: state,
            key: key
        }
        
    }

    pub fn tick(&mut self, id: String, data_path: String) -> Option<Track> {
        let item = mem::replace(self, UploadState::Finished(None));

        match item {
            UploadState::YoutubeDownload { ref state, key: _, ref downloader } => {
                let state = state.borrow();
                if state.progress >= 1.0 {
                    //TODO
                    mem::replace(self,
                        UploadState::converting_ffmpeg(downloader.handle.clone(), state.file.clone(), id.clone(), &state.get_content().unwrap(), state.format()));
                }

                None
            },
            UploadState::ConvertingFFMPEG { key: _, ref state, ref converter } => {
                let state = state.borrow();

                if state.progress >= 0.999 {
                    let (data, num_channel, duration) = state.read();

                    mem::replace(self,
                        UploadState::converting_opus(converter.handle.clone(), id.clone(), state.desc.clone(), &data, duration as f32, num_channel, data_path));
                }

                None
            },
            UploadState::ConvertingOpus { state, .. } => {
                let state = state.borrow();

                if state.progress >= 1.0 {
                    if let Some(ref track) = state.data {
                        mem::replace(self, UploadState::Finished(Some((id, state.desc.clone(), track.key.clone()))));

                        return Some(track.clone());
                    }
                }

                None
            },
            UploadState::Finished(Some(_)) => { mem::replace(self, UploadState::Finished(None)); None},
            _ => None
        }
    }

    pub fn should_retain(&self) -> bool {
        match self {
            UploadState::Finished(None) => false,
            _ => true
        }
    }

    pub fn kind(&self) -> &str {
        match *self {
            UploadState::YoutubeDownload { .. } => "youtube_download",
            UploadState::ConvertingFFMPEG { .. } => "converting_ffmpeg",
            UploadState::ConvertingOpus { .. } => "converting_opus",
            UploadState::Finished(_) => "finished"
        }
    }
    pub fn progress(&self) -> f32 {
        match *self {
            UploadState::YoutubeDownload { ref state, .. } => state.borrow().progress,
            UploadState::ConvertingFFMPEG { ref state, .. } => state.borrow().progress,
            UploadState::ConvertingOpus { ref state, .. } => state.borrow().progress,
            UploadState::Finished(_) => 1.0
        }
    }
    pub fn key(&self) -> String {
        match *self {
            UploadState::YoutubeDownload { ref key, .. } => key.clone(),
            UploadState::ConvertingFFMPEG { ref key, .. } => key.clone(),
            UploadState::ConvertingOpus { ref key, .. } => key.clone(),
            UploadState::Finished(Some((ref key, _, _))) => key.clone(),
            _ => "".into()
        }
    }

    pub fn track_key(&self) -> Option<String> {
        match self {
            UploadState::YoutubeDownload { .. } => None,
            UploadState::ConvertingFFMPEG { .. } => None,
            UploadState::ConvertingOpus { .. } => None,
            UploadState::Finished(Some((_, _, ref track_key))) => Some(track_key.clone()),
            UploadState::Finished(None) => None
        }
    }
    pub fn desc(&self) -> String {
        match self {
            UploadState::YoutubeDownload { ref state, .. } => state.borrow().file.clone(),
            UploadState::ConvertingFFMPEG { ref state, .. } => state.borrow().desc.clone(),
            UploadState::ConvertingOpus { ref state, .. } => state.borrow().desc.clone(),
            UploadState::Finished(Some((_, ref desc, _))) => desc.clone(),
            _ => "".into()
        }
    }

}
