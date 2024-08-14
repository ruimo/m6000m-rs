use std::{fs::File, io::{self, BufReader, Cursor, Read, Write}, sync::mpsc::{self, TryRecvError}, thread};
use rodio::Source;
use serde_jsonlines::WriteExt;
use crate::{arg, format::Jsonl};
use log::{error, warn, info, debug};

pub trait DataSubscriber {
  /// Implement this method to handle received data.
  fn on_data(&mut self, data: &es51986::Output);
}

/// A DataSubscriber that reports data to stdout.
pub struct StdoutDataSubscriber {
  format: arg::OutputFormat,
}

impl StdoutDataSubscriber {
  pub fn new(format: arg::OutputFormat) -> Self {
    Self { format }
  }
}

impl DataSubscriber for StdoutDataSubscriber {
  fn on_data(&mut self, data: &es51986::Output) {
    match self.format {
        arg::OutputFormat::Jsonl => {
            let json = Jsonl {
                value: data.get_value(),
                raw: data.clone(),
            };
            io::stdout().write_json_lines(vec![json]).unwrap();
        }
    }
  }
}

pub struct VoiceboxDataSubscriber {
  url: String,
  speaker: usize,
  tx: mpsc::Sender<String>,
}

#[derive(Debug, Clone, PartialEq)]
enum VoiceboxDataSubscriberErr {
  Disconnected,
}

impl VoiceboxDataSubscriber {
  fn last_msg(rx: &mpsc::Receiver<String>) -> Result<String, VoiceboxDataSubscriberErr> {
    match rx.recv() {
        Ok(msg) => {
          let mut last_msg = msg;
          loop {
            match rx.try_recv() {
              Ok(next_msg) => last_msg = next_msg,
              Err(TryRecvError::Empty) => return Ok(last_msg),
              Err(TryRecvError::Disconnected) => return Err(VoiceboxDataSubscriberErr::Disconnected),
            }
          }
        },
        Err(_) => Err(VoiceboxDataSubscriberErr::Disconnected),
    }
  }
  
  fn speak(base_url: &str, speaker: usize, msg: String, device: &rodio::Device) {
    let url: reqwest::Url = match reqwest::Url::parse_with_params(
      &format!("{}/{}", base_url, "audio_query"),
      &[
      ("speaker", speaker.to_string()),
      ("text", msg.clone()),
      ]
    ) {
        Ok(url) => url,
        Err(err) => {
          error!("Parsing url {} failed(speaker = {}, text = {}): {}", err, base_url, speaker, msg);
          return;
        }
    };

    let resp: Result<reqwest::blocking::Response, reqwest::Error> = reqwest::blocking::Client::new()
    .post(url.clone())
    .send();
    
    let json = match resp {
        Ok(resp) => {
          match resp.text() {
            Ok(json) => json,
            Err(err) => {
              error!("Cannot parse response: {:?}", err);
              return;
            }
          }
        }
        Err(err) => {
          error!("Request to url {:?} failed: {:?}", url, err);
          return;
        }
    };

    let url: reqwest::Url = match reqwest::Url::parse_with_params(
      &format!("{}/{}", base_url, "synthesis"),
      &[
      ("speaker", speaker.to_string()),
      ]
    ) {
        Ok(url) => url,
        Err(err) => {
          error!("Parsing url {} failed(speaker = {}): {}", err, url, speaker);
          return;
        }
    };

    let resp: Result<reqwest::blocking::Response, reqwest::Error> = reqwest::blocking::Client::new()
    .post(url.clone())
    .header("Content-Type", "application/json")
    .body(json)
    .send();

    let wav: bytes::Bytes = match resp {
        Ok(resp) => {
          match resp.bytes() {
            Ok(wav) => wav,
            Err(err) => {
              error!("Cannot parse response: {:?}", err);
              return;
            }
          }
        }
        Err(err) => {
          error!("Request to url {:?} failed: {:?}", url, err);
          return;
        }
      };

      let wav: Vec<u8> = wav.into();
      let (_stream, stream_handle) = match rodio::OutputStream::try_from_device(device) {
        Ok(ok) => ok,
        Err(err) => {
          error!("Cannot open device {:?}", err);
          return;
        }
      };
      let source = match rodio::Decoder::new_wav(Cursor::new(wav)) {
        Ok(ok) => ok,
        Err(err) => {
          error!("Cannot decode wav file: {:?}", err);
          return;
        }
      };
      let sink = match rodio::Sink::try_new(&stream_handle) {
        Ok(ok) => ok,
        Err(err) => {
          error!("Cannot create sink device {:?}", err);
          return;
        }
      };
      sink.append(source);
      sink.sleep_until_end();
  }

  pub fn new(url: String, speaker: usize, device: rodio::Device) -> Self {
    let url: String = if url.ends_with("/") {
      url[..url.len() - 1].to_owned()
    } else {
      url
    };
    let (tx, rx): (mpsc::Sender<String>, mpsc::Receiver<String>) = mpsc::channel();
    let cloned_url = url.clone();
    thread::spawn(move || loop {
      match Self::last_msg(&rx) {
        Ok(msg) => Self::speak(&cloned_url, speaker, msg, &device),
        Err(VoiceboxDataSubscriberErr::Disconnected) => {
          warn!("Voicebox thread disconnected.");
          break;
        }
      }
    });

    Self { url, tx, speaker }
  }
}

impl DataSubscriber for VoiceboxDataSubscriber {
    fn on_data(&mut self, data: &es51986::Output) {
      let value: Option<es51986::OutputValue> = data.get_value();
      if let Some(value) = value {
        let prefix_unit = match &value.value_unit.prefix_unit {
            es51986::PrefixUnit::Mega => "メガ",
            es51986::PrefixUnit::Kilo => "キロ",
            es51986::PrefixUnit::None => "ナノ",
            es51986::PrefixUnit::Millis => "ミリ",
            es51986::PrefixUnit::Micro => "マイクロ",
            es51986::PrefixUnit::Nano => "ナノ",
        };
        let base_unit = match &value.value_unit.base_unit {
            es51986::BaseUnit::Ampere => "アンペア",
            es51986::BaseUnit::Volt => "ボルト",
            es51986::BaseUnit::Ohm => "オーム",
            es51986::BaseUnit::Hearts => "ヘルツ",
            es51986::BaseUnit::Farad => "ファラッド",
        };
        self.tx.send(format!("{}{}{}", value.digits, prefix_unit, base_unit)).unwrap();
      }
    }
}