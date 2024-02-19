use std::{
    collections::HashMap,
    sync::{atomic::AtomicBool, mpsc::Receiver, Arc},
    time::Duration,
};

use serde::Deserialize;
use vulture::{MemoryScanner, ProcessHandle};

type Res<T> = Result<T, Box<dyn std::error::Error>>;

fn main() -> Res<()> {
    // once a change is detected then rerun the scanner
    let args: Vec<_> = std::env::args().collect();
    let proc_id = args[1].parse().unwrap();

    let (sender, reciever) = std::sync::mpsc::channel::<ChannelMsg>();

    let scanner_status = Arc::new(AtomicBool::new(false));

    let scanner_status_clone = scanner_status.clone();
    // start a background thread to handle the memory scanning
    std::thread::spawn(move || {
        let process = ProcessHandle::new(proc_id);
        handle_scanner(process, reciever, scanner_status_clone).unwrap();
    });

    let mut bbset = BBSet::new();

    loop {
        std::thread::sleep(Duration::from_secs(5));
        if scanner_status.load(std::sync::atomic::Ordering::Relaxed) {
            println!("scanner is still working.. sleeping for another loop");
            continue;
        }

        match get_ocr_records() {
            Ok(records) => {
                println!("got {} records", records.len());

                for record in records {
                    match bbset.add(&record) {
                        Some((true, bbox)) => {
                            sender.send(ChannelMsg::ReScan(bbox))?;
                        }
                        Some((false, bbox)) => {
                            sender.send(ChannelMsg::InitScan(bbox))?;
                        }
                        None => (),
                    }
                }
            }
            Err(e) => {
                eprintln!("failed to get records. {e:?}")
            }
        }
    }
}

fn get_screen() -> Res<Vec<u8>> {
    println!("making request for screen data");
    let endpoint = std::env::var("SCREEN_ENDPOINT")?;
    let resp = reqwest::blocking::get(endpoint)?;
    let bytes = resp.bytes()?;
    println!("got reponse for screen data");
    Ok(bytes.to_vec())
}

fn get_ocr_records() -> Res<Vec<BBoxItem>> {
    println!("making request for ocr data");
    let endpoint = std::env::var("OCR_ENDPOINT")?;
    let resp = reqwest::blocking::Client::new()
        .post(endpoint)
        .body(get_screen()?)
        .send()?
        .bytes()?;
    println!("got reponse for ocr data");
    let resp_body = std::str::from_utf8(&resp)?;
    let bboxes = serde_json::from_str::<Vec<OCRWord>>(resp_body)?
        .into_iter()
        .map(Into::into)
        .collect();
    Ok(bboxes)
}

fn handle_scanner(
    process: ProcessHandle,
    rx: Receiver<ChannelMsg>,
    running: Arc<AtomicBool>,
) -> Res<()> {
    let mut map = HashMap::new();

    while let Ok(msg) = rx.recv() {
        println!("[scanner] recieved message!");
        running.store(true, std::sync::atomic::Ordering::Relaxed);
        let res = match msg {
            ChannelMsg::InitScan(word) => {
                let scanner = map
                    .entry(word.id)
                    .or_insert_with(|| vulture::DefaultScanner::new(process.clone()));

                println!("[scanner] starting init scan");
                scanner.find_values(
                    &word.text.parse::<f64>().unwrap(),
                    |a, b| a == b,
                    vulture::ScanType::Initialize,
                )?
            }
            ChannelMsg::ReScan(word) => {
                let scanner = map
                    .entry(word.id)
                    .or_insert_with(|| vulture::DefaultScanner::new(process.clone()));

                println!("[scanner] starting re-scan");
                scanner.find_values(
                    &word.text.parse::<f64>().unwrap(),
                    |a, b| a == b,
                    vulture::ScanType::Prune,
                )?
            }
        };

        println!("[scanner] scanning response: {:?}", res);
        running.store(false, std::sync::atomic::Ordering::Relaxed);
    }

    Ok(())
}
enum ChannelMsg {
    InitScan(BBoxItem),
    ReScan(BBoxItem),
}

#[derive(Clone, Deserialize)]
struct BBoxItem {
    id: String,
    x1: u32,
    y1: u32,
    x2: u32,
    y2: u32,
    text: String,
}

#[derive(Clone, Deserialize)]
struct OCRWord {
    x1: u32,
    y1: u32,
    x2: u32,
    y2: u32,
    text: String,
}

impl Into<BBoxItem> for OCRWord {
    fn into(self) -> BBoxItem {
        BBoxItem {
            id: format!("{}{}{}{}", self.x1, self.x2, self.y1, self.y2),
            x1: self.x1,
            y1: self.y1,
            x2: self.x2,
            y2: self.y2,
            text: self.text,
        }
    }
}

// data structure to hold and fuzzy match bounding boxes
struct BBSet {
    theshhold: u32,
    items: Vec<BBoxItem>,
}
impl BBSet {
    pub fn new() -> Self {
        Self {
            theshhold: 5,
            items: Vec::new(),
        }
    }

    #[inline]
    fn within_bound(v1: u32, v2: u32, thresh: u32) -> bool {
        (v1 < v2 + thresh) || (v1 > v2 - thresh)
    }

    pub fn add(&mut self, bbox: &BBoxItem) -> Option<(bool, BBoxItem)> {
        if let Some(ref mut word) = self.items.iter_mut().find(|s| {
            Self::within_bound(s.x1, bbox.x1, self.theshhold)
                && Self::within_bound(s.x2, bbox.x2, self.theshhold)
                && Self::within_bound(s.y1, bbox.y1, self.theshhold)
                && Self::within_bound(s.y2, bbox.y2, self.theshhold)
        }) {
            // when a value is updated, create a new scan call for it.
            if word.text != bbox.text {
                word.text = bbox.text.to_string();
                return Some((true, word.clone()));
            }
            None
        } else {
            self.items.push(bbox.clone());
            Some((false, bbox.clone()))
        }
    }
}
