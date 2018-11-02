pub mod ffmpeg;
pub mod youtube;
pub mod opus;
pub mod download;

use std::mem;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::cell::RefCell;

use tokio_core::reactor::Handle;
use futures::{Stream, Future, IntoFuture};

use hex_database::{Track, TrackKey};
use hex_server_protocol::PacketId;

pub use self::download::DownloadState;

pub enum UploadState {
    YoutubeDownload {
        downloader: youtube::Downloader,
        state: Rc<RefCell<youtube::State>>,
        id: PacketId
    },
    ConvertingFFMPEG {
        converter: ffmpeg::Converter,
        state: Rc<RefCell<ffmpeg::State>>,
        id: PacketId
    },
    ConvertingOpus {
        converter: opus::Converter,
        state: Rc<RefCell<opus::State>>,
        id: PacketId
    },
    Finished(Option<(PacketId, String, TrackKey)>)
}

impl UploadState {
    pub fn youtube(id: PacketId, path: &str, handle: Handle) -> UploadState {
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
            id: id
        }
    }

    pub fn converting_ffmpeg(handle: Handle, desc: String, id: PacketId, data: &[u8], format: &str) -> UploadState {
        let mut dwnd = ffmpeg::Converter::new(handle.clone(), desc.clone(), data, format).unwrap();

        let state = Rc::new(RefCell::new(ffmpeg::State::empty(desc, PathBuf::new(), PathBuf::new())));
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
            id: id
        }
    }

    pub fn converting_opus(handle: Handle, id: PacketId, desc: String, samples: &[i16], duration: f32, num_channel: u32, data_path: PathBuf) -> UploadState {
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
            id: id
        }
        
    }

    pub fn tick(&mut self, data_path: PathBuf) -> Option<Track> {
        let item = mem::replace(self, UploadState::Finished(None));

        let (next, ret): (Option<UploadState>, Option<Track>) = match &item {
            UploadState::YoutubeDownload { ref state, ref id, ref downloader } => {
                let state = state.borrow();
                if state.progress >= 1.0 {
                    //TODO
                    (Some(UploadState::converting_ffmpeg(downloader.handle.clone(), state.file.clone(), id.clone(), &state.get_content().unwrap(), state.format())), None)
                } else {
                    (None, None)
                }

            },
            UploadState::ConvertingFFMPEG { ref id, ref state, ref converter } => {
                let state = state.borrow();

                if state.progress >= 0.999 {
                    let (data, num_channel, duration) = state.read();

                    (Some(UploadState::converting_opus(converter.handle.clone(), id.clone(), state.desc.clone(), &data, duration as f32, num_channel, data_path)), None)
                } else {
                    (None, None)
                }
            },
            UploadState::ConvertingOpus { state, id, .. } => {
                let state = state.borrow();

                if state.progress >= 1.0 {
                    if let Some(ref track) = state.data {
                        (Some(UploadState::Finished(Some((id.clone(), state.desc.clone(), track.key.clone())))), Some(track.clone()))
                    } else {
                        (None, None)
                    }
                } else {
                    (None, None)
                }
            },
            UploadState::Finished(Some(_)) => { (Some(UploadState::Finished(None)), None)},
            _ => (None, None)
        };

        match next {
            Some(x) => {mem::replace(self, x);},
            None => {mem::replace(self, item);}
        }

        ret
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
    pub fn id(&self) -> Option<PacketId> {
        match *self {
            UploadState::YoutubeDownload { ref id, .. } => Some(id.clone()),
            UploadState::ConvertingFFMPEG { ref id, .. } => Some(id.clone()),
            UploadState::ConvertingOpus { ref id, .. } => Some(id.clone()),
            UploadState::Finished(Some((ref id, _, _))) => Some(id.clone()),
            UploadState::Finished(None) => None
        }
    }

    pub fn track_key(&self) -> Option<TrackKey> {
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
