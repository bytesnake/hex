use cpal;
use std::thread;
use rb::{SpscRb, RB, RbProducer, RbConsumer, Producer, Consumer};

pub struct AudioDevice {
    format: cpal::Format,
    rb: SpscRb<i16>,
    producer: Producer<i16>,
    thread_handle: thread::JoinHandle<()>
}

impl AudioDevice {
    pub fn new() -> AudioDevice {
        let rb = SpscRb::new(4096);
        let (prod, cons) = (rb.producer(), rb.consumer());

        let device = cpal::default_output_device().expect("Failed to get default output device");

        if device.supported_output_formats().unwrap().filter(|x| x.channels == 2 && x.data_type == cpal::SampleFormat::I16).count() == 0 {
            panic!("No suitable device found!");
        }

        let format = cpal::Format {
            channels: 2,
            sample_rate: cpal::SampleRate(48000),
            data_type: cpal::SampleFormat::I16
        };

        let format2 = format.clone();
        let thread = thread::spawn(move || Self::run(cons, device, format2));

        AudioDevice {
            format: format,
            rb: rb,
            producer: prod,
            thread_handle: thread
        }
    }

    pub fn buffer(&mut self, buf: &[i16]) {
        self.producer.write_blocking(buf).expect("Couldn't queue block to buffer");
    }

    pub fn format(&self) -> cpal::Format {
        self.format.clone()
    }

    pub fn run(consumer: Consumer<i16>, device: cpal::Device, format: cpal::Format) {
        let event_loop = cpal::EventLoop::new();

        let stream_id = event_loop.build_output_stream(&device, &format).unwrap();
        event_loop.play_stream(stream_id.clone());

        let mut buf = vec![0i16; format.channels as usize];

        event_loop.run(move |_, data| {
            println!("Loop");
            match data {
                cpal::StreamData::Output { buffer: cpal::UnknownTypeOutputBuffer::I16(mut buffer) } => {
                    for sample in buffer.chunks_mut(format.channels as usize) {
                        let cnt = consumer.read_blocking(&mut buf);

                        let mut i = 0;
                        for out in sample.iter_mut() {
                            *out = buf[i];

                            i += 1;

                        }
                    }
                },
                _ => {}
            }
        });
    }
}
