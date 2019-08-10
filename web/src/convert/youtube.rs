use std::io::{self, Read};
use std::fs::{self, File};
use std::process::Command;
use std::process::Stdio;
use std::result;

use tokio_io::AsyncRead;
use tokio_codec;
use tokio_process::{Child, ChildStderr, ChildStdout, CommandExt};
use tokio_core::reactor::Handle;

use futures::{Future, Poll, Stream, IntoFuture};
use bytes::BytesMut;

use crate::error::{Result, Error};

struct LineCodec;

// straight from
// https://github.com/tokio-rs/tokio-line/blob/master/simple/src/lib.rs
impl tokio_codec::Decoder for LineCodec {
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
pub struct ToLine<T>(tokio_codec::FramedRead<T, LineCodec>);

impl<T: AsyncRead> ToLine<T> {
    fn new(stderr: T) -> Self {
        ToLine(tokio_codec::FramedRead::new(stderr, LineCodec {}))
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
pub enum StateError {
    IO(String)
}

#[derive(Clone, Debug, Serialize)]
pub struct State {
    name: String,
    pub file: String,
    pub progress: f32
}

impl State {
    pub fn empty() -> State {
        State {
            name: "".into(),
            file: "".into(),
            progress: 0.0
        }
    }

    pub fn format(&self) -> &str {
        self.file.rsplit(".").next().unwrap()
    }

    pub fn get_content(&self) -> Result<Vec<u8>> {
        let mut file = File::open(&self.file)
            .map_err(|_| Error::ConvertYoutube)?;

        let mut tmp = Vec::new();
        file.read_to_end(&mut tmp).unwrap();

        fs::remove_file(&self.file)
            .map_err(|_| Error::ConvertYoutube)?;
        //debug!("Read {} bytes from youtube", nread);

        Ok(tmp)
    }
}   

pub struct Downloader {
    pub handle: Handle,
    child: Option<Child>,
    stdout: Option<ToLine<ChildStdout>>,
    stderr: Option<ToLine<ChildStderr>>
}

impl Downloader {
    pub fn new(handle: Handle, addr: &str) -> Downloader {
        let mut cmd = Command::new("unbuffer")
            .arg("youtube-dl")
            .arg("--external-downloader").arg("aria2c")
            .arg("-f").arg("bestaudio")
            .arg("-o").arg("/tmp/%(title)s.%(ext)s")
            .arg(addr)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn_async(&handle)
            .expect("Failed to spawn youtube-dl!");

        let (stdout, stderr) = (cmd.stdout().take().unwrap(), cmd.stderr().take().unwrap());

        Downloader {
            handle: handle,
            child: Some(cmd),
            stdout: Some(ToLine::new(stdout)),
            stderr: Some(ToLine::new(stderr))
        }
    }

    pub fn state(&mut self) -> impl Stream<Item=State, Error=StateError> {
        if let (Some(out), Some(err)) = (self.stdout.take(), self.stderr.take()) {
            let mut state = State::empty();

            out.chain(err).map(move |msg| {
                println!("Msg: {}", msg);

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
                }


                state.clone()
            }).map_err(|_| {
                StateError::IO("".into())
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
