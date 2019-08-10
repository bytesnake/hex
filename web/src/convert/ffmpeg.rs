use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::fs::{File, self};
use std::process::Command;
use std::process::Stdio;
use std::{result, slice};

use tokio_io::AsyncRead;
use tokio_codec;
use tokio_process::{Child, ChildStderr, ChildStdout, CommandExt};
use tokio_core::reactor::Handle;

use futures::{Future, Stream, IntoFuture};
use bytes::BytesMut;

use crate::error::{Result, Error};
use tempfile::NamedTempFile;

struct LineCodec;

// straight from
// https://github.com/tokio-rs/tokio-line/blob/master/simple/src/lib.rs
impl tokio_codec::Decoder for LineCodec {
    type Item = String;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> result::Result<Option<String>, io::Error> {
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

/*impl<T: AsyncRead> Stream for ToLine<T> {
    type Item = String;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        println!("Poll");
        self.0.poll()
    }
}*/

#[derive(Debug)]
pub enum StateError {
    IO(String)
}

#[derive(Clone, Debug, Serialize)]
pub struct State {
    file_in: PathBuf,
    file_raw: PathBuf,
    duration: Option<u64>,
    pub desc: String,
    pub progress: f32
}

impl State {
    pub fn empty(desc: String, file_in: PathBuf, file_raw: PathBuf) -> State {
        State {
            file_in: file_in,
            file_raw: file_raw,
            duration: None,
            desc: desc,
            progress: 0.0
        }
    }

    pub fn read(&self) -> (Vec<i16>, u32, f64) {
        let mut file = File::open(&self.file_raw).unwrap();

        let mut pcm = vec![];

        file.read_to_end(&mut pcm).unwrap();

        fs::remove_file(&self.file_raw).unwrap();
        fs::remove_file(&self.file_in).unwrap();

        let pcm: &[i16] = unsafe {
            slice::from_raw_parts(
                pcm.as_ptr() as *const i16,
                pcm.len() / 2
            )
        };

        (pcm.into(), 2, pcm.len() as f64 / 2.0 / 48000.0)
    }
}   

pub struct Converter {
    pub handle: Handle,
    file_in: NamedTempFile,
    file_raw: NamedTempFile,
    desc: String,
    child: Option<Child>,
    stdout: Option<ToLine<ChildStdout>>,
    stderr: Option<ToLine<ChildStderr>>
}

fn duration_to_time(inp: &str) -> Option<u64> {
    let mut groups = inp.split(":");
    if let (Some(hour), Some(min), Some(secs_mill)) = (groups.next(), groups.next(), groups.next()) {
        let mut groups = secs_mill.split(".");
        if let (Some(sec), Some(mil)) = (groups.next(), groups.next()) {
            if let (Ok(hour), Ok(min), Ok(sec), Ok(mil)) = (hour.parse::<u64>(), min.parse::<u64>(), sec.parse::<u64>(), mil.parse::<u64>()) {
                return Some((60*60*hour + 60*min + sec) * 1000 + mil);
            }
        }
    }

    None
}
             

impl Converter {
    pub fn new(handle: Handle, desc: String, data: &[u8], format: &str) -> Result<Converter> {
        // Generate a new filename for our temporary conversion
        let mut file_in = NamedTempFile::new()
            .map_err(|_| Error::ConvertFFMPEG)?;
        let file_raw = NamedTempFile::new()
            .map_err(|_| Error::ConvertFFMPEG)?;

        file_in.write_all(data)
            .map_err(|_| Error::ConvertFFMPEG)?;

        //file_in.sync_all()
        //    .map_err(|_| Error::ConvertFFMPEG)?;

        let mut cmd = Command::new("unbuffer")
            .arg("ffmpeg")
            .arg("-y")
            .arg("-i").arg(file_in.path())
            .arg("-ar").arg("48000")
            .arg("-ac").arg("2")
            .arg("-f").arg("s16le")
            .arg(&file_raw.path())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn_async(&handle)
            .expect("Failed to spawn youtube-dl!");

        let (stdout, stderr) = (cmd.stdout().take().unwrap(), cmd.stderr().take().unwrap());

        Ok(Converter {
            handle: handle,
            desc: desc,
            file_in: file_in,
            file_raw: file_raw,
            child: Some(cmd),
            stdout: Some(ToLine::new(stdout)),
            stderr: Some(ToLine::new(stderr))
        })
    }

    pub fn state(&mut self) -> impl Stream<Item=State, Error=StateError> {
        if let (Some(out), Some(err)) = (self.stdout.take(), self.stderr.take()) {
            let mut state = State::empty(self.desc.clone(), PathBuf::from(self.file_in.path()), PathBuf::from(self.file_raw.path()));

            out.0.chain(err.0).map(move |msg| {
                println!("Msg: {}", msg);
                
                if msg.contains("Duration: ") {
                    if let Some(dur) = msg.trim().split(" ").skip(1).next()
                        .and_then(|x| x.split(",").next()).map(|x| x.trim()) {
                            state.duration = duration_to_time(dur);
                    }
                } else if msg.contains("time=") {
                    if let Some(time) = msg.split("time=").skip(1).next()
                        .and_then(|x| x.split(" ").next()).map(|x| x.trim()) {
                        
                            if let (Some(dur), Some(time)) = (state.duration, duration_to_time(time)) {
                                state.progress = time as f32 / dur as f32;

                                println!("Progress: {}", state.progress);
                            }
                    }

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
