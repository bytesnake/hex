use cpal;
use std::thread;
use rb::{SpscRb, RB, RbProducer, RbConsumer, Producer, Consumer};
use cpal::traits::HostTrait;
use cpal::traits::EventLoopTrait;
use cpal::traits::DeviceTrait;

pub struct AudioDevice {
    rb: SpscRb<i16>,
    producer: Producer<i16>,
}

impl AudioDevice {
    pub fn new() -> AudioDevice {
        let rb = SpscRb::new(48000 * 3);
        let (prod, cons) = (rb.producer(), rb.consumer());

        let host = cpal::default_host();
        let device = host.default_output_device().expect("Failed to get default output device");

        if device.supported_output_formats().unwrap().filter(|x| x.channels == 2 && x.data_type == cpal::SampleFormat::I16).count() == 0 {
            panic!("No suitable device found!");
        }

        let format = cpal::Format {
            channels: 2,
            sample_rate: cpal::SampleRate(48000),
            data_type: cpal::SampleFormat::I16
        };

        thread::spawn(move || Self::run(cons, host, device, format));

        AudioDevice {
            rb: rb,
            producer: prod
        }
    }

    pub fn buffer(&mut self, buf: &[i16]) {
        let mut written = 0;
        loop {
            let n = self.producer.write_blocking(&buf[written..]).expect("Couldn't queue block to buffer");

            written += n;

            if written == buf.len() {
                break;
            }
        }
    }

    pub fn clear(&mut self) {
        self.rb.clear();
    }

    /*pub fn format(&self) -> cpal::Format {
        self.format.clone()
    }*/

    pub fn run(consumer: Consumer<i16>, host: cpal::Host, device: cpal::Device, format: cpal::Format) {
        let event_loop = host.event_loop();

        let stream_id = event_loop.build_output_stream(&device, &format).unwrap();
        event_loop.play_stream(stream_id.clone()).unwrap();

        let mut buf = vec![0i16; format.channels as usize];

        event_loop.run(move |_, data| {
            match data {
                Ok(cpal::StreamData::Output { buffer: cpal::UnknownTypeOutputBuffer::I16(mut buffer) }) => {
                    for sample in buffer.chunks_mut(format.channels as usize) {
                        let _ = consumer.read_blocking(&mut buf);

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
