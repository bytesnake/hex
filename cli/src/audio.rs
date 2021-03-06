use cpal;
use std::thread;
use std::panic;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use rb::{SpscRb, RB, RbProducer, RbConsumer, Producer, Consumer};
use cpal::traits::HostTrait;
use cpal::traits::EventLoopTrait;
use cpal::traits::DeviceTrait;

pub struct AudioDevice {
    rb: SpscRb<i16>,
    producer: Producer<i16>,
    is_running: Arc<AtomicUsize>
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

        let is_running = Arc::new(AtomicUsize::new(2));
        let tmp = is_running.clone();
        thread::spawn(move || Self::run(cons, host, device, format, tmp));

        AudioDevice {
            rb: rb,
            producer: prod,
            is_running
        }
    }

    pub fn buffer(&mut self, buf: &[i16], written: usize) -> usize {
        self.producer.write(&buf[written..]).unwrap_or(0)
    }

    pub fn clear(&mut self) {
        self.rb.clear();
    }

    pub fn shutdown(self) {
        self.is_running.store(0, Ordering::Relaxed);
    }

    pub fn pause(&self) {
        self.is_running.store(1, Ordering::Relaxed);
    }

    pub fn cont(&self) {
        self.is_running.store(2, Ordering::Relaxed);
    }

    pub fn run(consumer: Consumer<i16>, host: cpal::Host, device: cpal::Device, format: cpal::Format, is_running: Arc<AtomicUsize>) {
        let event_loop = host.event_loop();

        let stream_id = event_loop.build_output_stream(&device, &format).unwrap();
        event_loop.play_stream(stream_id.clone()).unwrap();

        let mut buf = vec![0i16; format.channels as usize];

        event_loop.run(move |_, data| {
            if is_running.load(Ordering::Relaxed) == 0 {
                panic!("ignore");
            }

            match data {
                Ok(cpal::StreamData::Output { buffer: cpal::UnknownTypeOutputBuffer::I16(mut buffer) }) => {
                    if is_running.load(Ordering::Relaxed) == 1 {
                        for sample in buffer.chunks_mut(format.channels as usize) {
                            for out in sample.iter_mut() {
                                *out = 0;
                            }
                        }
                    } else {
                        for sample in buffer.chunks_mut(format.channels as usize) {
                            let _ = consumer.read_blocking(&mut buf);

                            let mut i = 0;
                            for out in sample.iter_mut() {
                                *out = buf[i];

                                i += 1;

                            }
                        }
                    }
                },
                _ => {}
            }
        });
    }
}
