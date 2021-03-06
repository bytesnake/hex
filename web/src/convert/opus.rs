use std::path::PathBuf;
use std::thread;
use std::fs::File;

use futures::{IntoFuture, Future, Stream};
use futures::sync::mpsc::{channel, Sender, Receiver};
use tokio_core::reactor::Handle;

use crate::error::{Result, Error};

use hex_music_container::{Container, Configuration};

use hex_database::Track;

pub struct State {
    pub progress: f32,
    pub desc: String,
    pub data: Option<Track>
}

impl State {
    pub fn empty(desc: String) -> State {
        State {
            progress: 0.0,
            desc: desc,
            data: None
        }
    }
}

fn worker(mut sender: Sender<State>, desc: String, samples: Vec<i16>, duration: f32, num_channel: u32, data_path: PathBuf) -> Result<Track> {
    //loop {
        // calculate the acousticid of the file
    
    let fingerprint = hex_database::utils::get_fingerprint(num_channel as u16, &samples)
        .map_err(|_| Error::AcousticID)?;

    let track = Track::empty(fingerprint, duration.into());

    let file = File::create(data_path.join(track.key.to_path())).unwrap();

    sender.try_send(State { progress: 0.0, desc: desc, data: None })
        .map_err(|_| Error::ChannelFailed)?;

    // TODO realtime
    Container::save_pcm(Configuration::Stereo, samples, file, None)
        .map_err(|err| Error::MusicContainer(err))?;

    Ok(track)
}

pub struct Converter {
    pub handle: Handle,
    recv: Option<Receiver<State>>
}

impl Converter {
    pub fn new(handle: Handle, desc: String, samples: Vec<i16>, duration: f32, num_channel: u32, data_path: PathBuf) -> Converter {
        let (sender, recv) = channel(10);

        thread::spawn(move || {
            let mut sender2 = sender.clone();
            let res = worker(sender, desc.clone(), samples, duration, num_channel, data_path).unwrap();

            sender2.try_send(State { progress: 1.0, desc: desc, data: Some(res) })
                .map_err(|_| Error::ChannelFailed).unwrap();
        });

        Converter {
            handle: handle,
            recv: Some(recv),
        }
    }

    pub fn state(&mut self) -> impl Stream<Item=State, Error=()> {
        if let Some(recv) = self.recv.take() {
            return recv;
        } else {
            panic!("Call just once");
        }
    }

    pub fn spawn<T>(&self, hnd: T)
    where T: Stream + 'static {
        self.handle.spawn(hnd.for_each(|_| Ok(())).into_future().map(|_| ()).map_err(|_| ()));
    }


}
