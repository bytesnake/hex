use std::thread;
use std::io::Write;

use futures::{IntoFuture, Future, Stream};
use futures::sync::mpsc::{channel, Sender, Receiver};
use tokio_core::reactor::Handle;

use error::{Result, ErrorKind};

pub struct State {
    pub progress: f32
}

impl State {
    pub fn empty() -> State {
        State {
            progress: 0.0,
        }
    }
}

fn worker(mut sender: Sender<State>, samples: Vec<i16>, duration: f32, num_channel: u32) {
    loop {
        sender.try_send(State::empty());

        thread::sleep_ms(1000);
    }
}

pub struct Converter {
    pub handle: Handle,
    recv: Option<Receiver<State>>,
    thread: thread::JoinHandle<()>
}

impl Converter {
    pub fn new(handle: Handle, samples: Vec<i16>, duration: f32, num_channel: u32) -> Converter {
        let (sender, recv) = channel(10);

        let thread = thread::spawn(move || worker(sender, samples, duration, num_channel));

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
