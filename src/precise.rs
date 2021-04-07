use std::process::{Command, Child, ChildStdout, ChildStdin, Stdio};
use std::io::Write;
use std::io::BufReader;
use std::io::BufRead;
use std::result::Result;
use bincode;


pub struct PreciseEngine {
    process: Child,
    model: String,
    chunk_size: usize,
    reader: BufReader<ChildStdout>,
    writer: ChildStdin
}


impl PreciseEngine {
    #[allow(dead_code)]
    pub fn stop(&mut self) {
        match self.process.kill() {
            Ok(_) => (),
            Err(_) => println!("Couldn't kill process :(")
        }
    }

    #[allow(dead_code)]
    pub fn get_prediction(self: &mut Self, audio_data: &[i16]) -> Result<bool, &'static str> {
        if audio_data.len() != self.chunk_size {
            Err("audio data length doesn't match the expected chunk size")
        }
        else {
            let bytes: Vec<u8> = bincode::serialize(&audio_data).unwrap();
            let buffer = &bytes[..];
            println!(">");
            self.writer.write_all(&buffer);
            self.writer.flush().unwrap();
            let mut buf = vec![];
            self.reader.read_until(b'\n', &mut buf);

            let line = match std::str::from_utf8(&buf) {
                Ok(v) => v,
                Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
            };
            println!("{}", line);
            Ok(false)
        }
    }

    #[allow(dead_code)]
    fn get_model(&self) -> String {
        self.model.clone()
    }
}

    
pub fn get_runner() -> PreciseEngine {
    
    let cmd = "/home/ake/.mycroft/precise/precise-engine/precise-engine";
    let model = "/home/ake/.mycroft/precise/hey-mycroft.pb";

    let mut child = Command::new(cmd).stdin(Stdio::piped())
        .arg(model)
        .arg("2048")
        .stdin(Stdio::piped())
//        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn().unwrap();

    let out = child.stdout.take().unwrap();
    let cmd_out = BufReader::new(out);

    let cmd_in = child.stdin.take().unwrap();

    let p = PreciseEngine {
        process: child,
        model: model.to_string(),
        chunk_size: 2048,
        reader: cmd_out,
        writer: cmd_in
    };
    p
}
