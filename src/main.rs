//! A listener client for Mycroft written in rust
//!
//! Audio from the default input device is sent to precise and then to STT

extern crate portaudio;
extern crate ringbuf;
extern crate hound;

use std::time::{Instant,Duration};
use ringbuf::{RingBuffer, Producer, Consumer};
use std::vec::Vec;
use hound::WavWriter;

use std::fs;
use std::io;
use std::mem;
use std::thread::sleep;

mod precise;

fn main() {
    let runner = precise::get_runner();
    sleep(Duration::new(3, 0));
    let pa = match open_audio_port() {
        Ok(port) => port,
        Err(error) => panic!("{}", String::from(error))
    };

    let rb = RingBuffer::<i16>::new(65536);
    let (sender, receiver) = rb.split();
    let mut stream = open_stream(&pa, sender).unwrap();

    match stream.start() {
        Ok(_) => {}
        Err(error) => panic!("{}", error.to_string()),
    };

    wait_for_wakeword(receiver, runner);
    record_for_stt();
    send_to_mycroft();
    match close_stream(stream){
        Ok(_) => {}
        Err(error) => panic!("{}", error.to_string()),
    };
}


fn wait_for_wakeword(mut receiver: Consumer<i16>, mut runner: precise::PreciseEngine) {
    let start = Instant::now();
    let time_to_wait = &(5 as u64);
    let mut input_data = Vec::new();

    let mut wav_writer = match get_wav_writer("recorded.wav", CHANNELS,
                                              SAMPLE_RATE) {
        Ok(writer) => writer,
        Err(error) => panic!("{}", error.to_string()),
    };

    while start.elapsed().as_secs().lt(time_to_wait) {
        match receiver.pop() {
            Some(sample) => {
                input_data.push(sample);
            },
            None => ()
        }
        if input_data.len() >= 2048 {
            let v = &input_data[0..2048];
            for sample in v.iter() {
                wav_writer.write_sample(*sample).unwrap();
            }
            runner.get_prediction(v);
            input_data.drain(0..2048);
        }
    }
    runner.stop()
}


fn record_for_stt() {
    println!("Recording for STT")
}

fn send_to_mycroft() {
    println!("Sending to Mycroft");
}


fn get_wav_writer(path: &'static str, channels: i32, sample_rate: f64) -> Result<WavWriter<io::BufWriter<fs::File>>,String> {
    let spec = wav_spec(channels, sample_rate);
    match hound::WavWriter::create(path, spec) {
        Ok(writer) => Ok(writer),
        Err(error) => Err (String::from(format!("{}",error))),
    }
}

fn wav_spec(channels: i32, sample_rate: f64) -> hound::WavSpec {
    hound::WavSpec {
        channels: channels as _,
        sample_rate: sample_rate as _,
        bits_per_sample: (mem::size_of::<i16>()*8) as _,
        sample_format: hound::SampleFormat::Int,
    }
}

fn close_stream(mut stream: portaudio::Stream<portaudio::NonBlocking, portaudio::Input<i16>>) -> Result<String, String> {
    match stream.stop() {
        Ok(_) => {
            Ok(String::from("Stream closed"))
        },
        Err(error) => {
            Err(error.to_string())
        },
    }
}


const CHANNELS: i32 = 1;
const SAMPLE_RATE: f64 = 16_000.0;
const FRAMES: u32 = 256;


fn open_stream(pa: &portaudio::PortAudio, mut sender: Producer<i16>) ->
                    Result<portaudio::Stream<portaudio::NonBlocking,
                                             portaudio::Input<i16>>, String> {
    let input_index = match get_input_device_index(&pa) {
        Ok(index) => index,
        Err(error) => return Err(String::from(error))
    };

    let mut wav_writer = match get_wav_writer("compare.wav", CHANNELS,
                                              SAMPLE_RATE) {
        Ok(writer) => writer,
        Err(error) => panic!("{}", error.to_string()),
    };

    let input_settings = match get_input_settings(input_index, &pa, SAMPLE_RATE, FRAMES, CHANNELS) {
        Ok(settings) => settings,
        Err(error) => return Err(String::from(error))
    };

    let callback = move |portaudio::InputStreamCallbackArgs { buffer, .. }| {
        for sample in buffer.iter() {
            sender.push(*sample).unwrap();
            wav_writer.write_sample(*sample).unwrap();
        }
        portaudio::Continue
    };

    // Construct a stream
    let stream = match pa.open_non_blocking_stream(input_settings, callback) {
        Ok(strm) => strm,
        Err(error) => return Err(error.to_string()),
    };
    Ok(stream)
}


fn open_audio_port() -> Result<portaudio::PortAudio, String>
{
    portaudio::PortAudio::new().or_else(|error| Err(String::from(format!("{}", error))))
}

fn get_input_device_index(pa: &portaudio::PortAudio) -> Result<portaudio::DeviceIndex, String>
{
    pa.default_input_device().or_else(|error| Err(String::from(format!("{}", error))))
}

fn get_input_latency(audio_port: &portaudio::PortAudio, input_index: portaudio::DeviceIndex) -> Result<f64, String>
{
    let input_device_information = audio_port.device_info(input_index).or_else(|error| Err(String::from(format!("{}", error))));
    Ok(input_device_information.unwrap().default_low_input_latency)
}

fn get_input_stream_parameters(input_index: portaudio::DeviceIndex, latency: f64, channels: i32) -> Result<portaudio::StreamParameters<i16>, String>
{
    const INTERLEAVED: bool = true;
    Ok(portaudio::StreamParameters::<i16>::new(input_index, channels, INTERLEAVED, latency))
}

fn get_input_settings(input_index: portaudio::DeviceIndex, pa: &portaudio::PortAudio, sample_rate: f64, frames: u32, channels: i32) -> Result<portaudio::InputStreamSettings<i16>, String>
{
    Ok(
        portaudio::InputStreamSettings::new(
            (get_input_stream_parameters(
                input_index,
                (get_input_latency(
                    &pa,
                    input_index,
                ))?,
                channels
            ))?,
            sample_rate,
            frames,
        )
    )
}
