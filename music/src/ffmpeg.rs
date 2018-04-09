use std::io::{self, Read, Write};
use std::fs::{self, File};
use std::process::Command;
use std::process::Stdio;
use std::result;

use tokio_io::{codec, AsyncRead, AsyncWrite};
use tokio_process::{Child, ChildStderr, ChildStdin, ChildStdout, CommandExt};
use tokio_core::reactor::Handle;

use futures::{Future, Poll, Stream, IntoFuture};
use bytes::BytesMut;

use failure::ResultExt;

use error::{Result, ErrorKind};
use uuid::Uuid;

use hound::WavReader;

struct LineCodec;

// straight from
// https://github.com/tokio-rs/tokio-line/blob/master/simple/src/lib.rs
impl codec::Decoder for LineCodec {
    type Item = String;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> result::Result<Option<String>, io::Error> {
        println!("{}", buf.len());

        if let Some(n) = buf.as_ref().iter().position(|b| (*b == b'\n' || *b == b'\r')) {
            let line = buf.split_to(n);
            buf.split_to(1);
            return match ::std::str::from_utf8(line.as_ref()) {
                Ok(s) => Ok(Some(s.to_string())),
                Err(_) => Err(io::Error::new(io::ErrorKind::Other, "invalid string")),
            };
        }
        Ok(None)
    }
}

/// A stream of Xi core stderr lines
pub struct ToLine<T>(codec::FramedRead<T, LineCodec>);

impl<T: AsyncRead> ToLine<T> {
    fn new(stderr: T) -> Self {
        ToLine(codec::FramedRead::new(stderr, LineCodec {}))
    }
}

impl<T: AsyncRead> Stream for ToLine<T> {
    type Item = String;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        self.0.poll()
    }
}

#[derive(Debug)]
pub enum Error {
    IO(String)
}

#[derive(Clone, Debug, Serialize)]
pub struct State {
    file: String,
    pub progress: f32
}

impl State {
    pub fn empty() -> State {
        State {
            file: "".into(),
            progress: 0.0
        }
    }

    pub fn read(&self) -> (Vec<i16>, u32, f64) {
        // read the whole wave file
        let mut reader = WavReader::open(&self.file).unwrap();

        let samples = reader.samples::<i16>().map(|x| x.unwrap()).collect::<Vec<i16>>();

        // use the metadata section to determine sample rate, number of channel and duration in
        // seconds
        let sample_rate = reader.spec().sample_rate as f64;
        let num_channel = reader.spec().channels;
        let duration = reader.duration() as f64 / sample_rate as f64;

        (samples, num_channel as u32, duration)
    }
}   

pub struct Converter {
    pub handle: Handle,
    child: Option<Child>,
    stdout: Option<ToLine<ChildStdout>>,
    stderr: Option<ToLine<ChildStderr>>
}

impl Converter {
    pub fn new(handle: Handle, data: &[u8], format: &str) -> Result<Converter> {
        // Generate a new filename for our temporary conversion
        let id = Uuid::new_v4();
        let filename = format!("/tmp/{}.{}", id, format);
        let filename_out = format!("/tmp/{}_out.{}", id, format);

        // convert to wave file
        let mut file = File::create(&filename)
            .context(ErrorKind::Conversion)?;

        file.write_all(data)
            .context(ErrorKind::Conversion)?;

        file.sync_all()
            .context(ErrorKind::Conversion)?;

        let mut cmd = Command::new("ffmpeg")
            .arg("-y")
            .arg("-i").arg(&filename)
            .arg("-ar").arg("48000")
            .arg(&filename_out)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn_async(&handle)
            .expect("Failed to spawn youtube-dl!");

        let (stdout, stderr) = (cmd.stdout().take().unwrap(), cmd.stderr().take().unwrap());

        Ok(Converter {
            handle: handle,
            child: Some(cmd),
            stdout: Some(ToLine::new(stdout)),
            stderr: Some(ToLine::new(stderr))
        })
    }

    pub fn state(&mut self) -> impl Stream<Item=State, Error=Error> {
        if let (Some(out), Some(err)) = (self.stdout.take(), self.stderr.take()) {
            let mut state = State::empty();

            out.chain(err).map(move |msg| {
                println!("Msg: {}", msg);

                /*
                if msg.contains("Destination: ") {
                    if let Some(file) = msg.split(": ").skip(1).next() {
                        let file = file.trim();
                        state.file = file.into();
                        state.name = file.into();
                    }
                } else if msg.contains("has already been") {
                    state.progress = 1.0;
                } else if msg.contains("%)") {
                    if let Some(pc) = msg.split("%)").next().and_then(|x| x.split("(").skip(1).next()).and_then(|x| x.trim().parse::<f32>().ok()) {
                        state.progress = pc / 100.0;
                    }
                } else if msg.contains("100%") {
                    state.progress = 1.0;
                }*/


                state.clone()
            }).map_err(|err| {
                Error::IO("".into())
            })

        } else {
            panic!("You can't call state twice!");
        }
    }

    pub fn spawn<T>(&self, hnd: T)
    where T: Stream + 'static {
        self.handle.spawn(hnd.for_each(|_| Ok(())).into_future().map(|_| ()).map_err(|_| ()));
    }

    pub fn child(&mut self) -> Child {
        self.child.take().unwrap()
    }
}
