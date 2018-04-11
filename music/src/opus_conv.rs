use std::thread;
use std::io::Write;
use std::mem;

use futures::{IntoFuture, Future, Stream};
use futures::sync::mpsc::{channel, Sender, Receiver};
use tokio_core::reactor::Handle;

use opus;
use opus::{Channels, Application};

use failure::ResultExt;
use error::{Result, ErrorKind};

use acousticid;
use database::Track;

use uuid::Uuid;

pub struct State {
    pub progress: f32,
    pub desc: String,
    pub data: Option<Result<(Track, Vec<u8>)>>
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

fn worker(mut sender: Sender<State>, desc: String, samples: Vec<i16>, duration: f32, num_channel: u32) -> Result<(Track, Vec<u8>)> {
    // calculate the acousticid of the file
    let fingerprint = acousticid::get_hash(num_channel as u16, &samples)?;
    let key = Uuid::new_v4();

    // now convert to the opus file format
    let channel = match num_channel {
        1 => Channels::Mono,
        _ => Channels::Stereo // TODO: more than two channels support
    };
        
    let mut opus_data: Vec<u8> = Vec::new();
    let mut tmp = vec![0u8; 4000];
    
    let mut encoder = opus::Encoder::new(48000, channel, Application::Audio).unwrap();
    
    let mut cnt = 0;
    let mut idx = 0;
    for i in samples.chunks(1920) {
        let nbytes: usize = {
            if i.len() < 1920 {
                let mut filled_up_buf = vec![0i16; 1920];
                filled_up_buf[0..i.len()].copy_from_slice(i);
    
                encoder.encode(&filled_up_buf, &mut tmp)
                    .context(ErrorKind::Conversion)?
            } else {
                encoder.encode(&i, &mut tmp)
                    .context(ErrorKind::Conversion)?
            }
        };
    
        //println!("Opus frame size: {}", nbytes);
    
        let nbytes_raw: [u8; 4] = unsafe { mem::transmute((nbytes as u32).to_be()) };
    
        opus_data.extend_from_slice(&nbytes_raw);
        opus_data.extend_from_slice(&tmp[0..nbytes]);
    
        idx += 1920;
        cnt = (cnt+1) % 10;
        if cnt == 0 {
            sender.try_send(State { progress: idx as f32 / samples.len() as f32, desc: desc.clone(), data: None });
        }
    }
    
    Ok((Track::empty(&key.simple().to_string(), &fingerprint, duration.into()), opus_data))
}

pub struct Converter {
    pub handle: Handle,
    recv: Option<Receiver<State>>,
    thread: thread::JoinHandle<()>
}

impl Converter {
    pub fn new(handle: Handle, desc: String, samples: Vec<i16>, duration: f32, num_channel: u32) -> Converter {
        let (sender, recv) = channel(10);

        let thread = thread::spawn(move || {
            let mut sender2 = sender.clone();
            let res = worker(sender, desc.clone(), samples, duration, num_channel);

            sender2.try_send(State { progress: 1.0, desc: desc, data: Some(res) });
        });

        Converter {
            handle: handle,
            recv: Some(recv),
            thread: thread
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
