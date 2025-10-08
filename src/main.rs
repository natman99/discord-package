use eframe::egui;
use egui::ScrollArea;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rayon::*;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use std::collections::hash_map::HashMap;
use std::f32;
use std::fs::{self, Metadata};
use std::io::{self, read_to_string};
use std::ops::Range;
use std::sync::{Arc, Mutex};
fn main() {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "My egui App",
        native_options,
        Box::new(|cc| Ok(Box::new(MyEguiApp::new(cc)))),
    )
    .unwrap();
}

#[derive(Default)]
struct MyEguiApp {
    messages: Vec<Message>,
    counted_words: Vec<SortedWord>,
    display: Vec<SortedWord>,
    only_text: bool,
    search: String,
    processed_word_count: usize,
    proessed_messages_count: usize,
    shown_words_count: usize
}

impl MyEguiApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        let time = std::time::Instant::now();
        let m = read_messages();
        let end = time.elapsed().as_millis();
        println!("Processed {} messages in {} miliseconds", m.len(), end);
        let proessed_messages_count = m.len();
        let counted_words = count_words(&m);
        let processed_word_count = counted_words.len();
        let display = counted_words.clone();
        let shown_words_count = display.len();
        println!("Displaying {} rows", counted_words.len());
        Self {
            messages: m,
            counted_words,
            display,
            search: String::new(),
            only_text: false,
            processed_word_count,
            shown_words_count,
            proessed_messages_count,
        }
        // Self::default()
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Discord messages!");
            ui.horizontal(|ui| {
                ui.label(format!("Processed messages: {}", self.proessed_messages_count));
                ui.label(format!("Processed words: {}", self.processed_word_count));
                ui.label(format!("Shown words: {}", self.display.iter().count()));
            });
            ui.horizontal(|ui| {
                ui.label("Search:");
                let search_box = ui.text_edit_singleline(&mut self.search);
                let only_text_check = ui.checkbox(&mut self.only_text, "Only allow alphanumeric");
                if search_box.changed() || only_text_check.changed() {
                    if self.search.trim().is_empty() {
                        self.display = self
                            .counted_words
                            .iter()
                            .filter(|f| {
                                if self.only_text == true {
                                    &f.only_text == &self.only_text
                                } else {
                                    true
                                }
                            })
                            .map(|f| f.clone())
                            .collect();
                    } else {
                        let d = self
                            .counted_words
                            .iter()
                            .filter(|f| f.word.chars().filter(|f|f.is_alphanumeric()).collect::<String>().contains(&self.search.trim()));
                        let mut o: Vec<SortedWord> = vec![];
                        d.for_each(|f| o.push(f.clone()));
                        self.display = o
                            .iter()
                            .filter(|f| {
                                if self.only_text == true {
                                    &f.only_text == &self.only_text
                                } else {
                                    true
                                }
                            })
                            .map(|f| f.clone())
                            .collect();
                    }
                }
            });
            let text_style = egui::TextStyle::Body;
            let row_height = ui.text_style_height(&text_style);
            ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height(ui.available_height())
                .show_rows(ui, row_height, self.display.len(), |ui, row_range| {
                    ui.set_min_height(ui.available_height());

                    for row in row_range {
                        let f = &self.display[row];
                        ui.horizontal(|ui| {
                            ui.label(format!("{}", f.word));
                            ui.separator();
                            ui.label(format!("{}", f.frequency));
                        });
                        ui.separator();
                    }
                })
        });
    }
}

fn read_messages() -> Vec<Message> {
    let mut message_files = vec![];
    let messages: Arc<Mutex<Vec<Message>>> = Arc::new(Mutex::new(vec![]));

    // root file
    let res = fs::read_dir("messages/");
    if let Err(_r) = res {
        return vec![];
    };
    let res = res.unwrap();
    // channel folders
    let folders: Vec<_> = res
        .into_iter()
        .filter(|f| {
            if let Ok(f) = f {
                f.metadata().unwrap().is_dir()
            } else {
                false
            }
        })
        .collect();

    let mut iterator: std::slice::Iter<'_, Result<fs::DirEntry, std::io::Error>> = folders.iter();
    // channel files
    while let Some(Ok(file)) = iterator.next() {
        let mut files = fs::read_dir(file.path()).unwrap();
        let f = files
            .find(|f| {
                f.as_ref()
                    .unwrap()
                    .file_name()
                    .to_str()
                    .unwrap()
                    .contains("messages.json")
            })
            .unwrap()
            .unwrap();
        message_files.push(f);
    }

    let mut strings = vec![];

    for i in message_files {
        let f = fs::read_to_string(i.path()).unwrap();
        strings.push(f);
    }
    // create message objects
    strings.par_iter().for_each(|f| {
        let mut s: Vec<Message> = serde_json::from_str(f).unwrap();
        messages.lock().unwrap().append(&mut s);
    });
    // println!("{:?}", messages);
    let mut out: Vec<Message> = Vec::new();
    out.append(&mut messages.lock().unwrap());

    out
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Message {
    #[serde(rename = "ID")]
    id: u64,
    timestamp: String,
    contents: String,
    attachments: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash, Eq)]
struct SortedWord {
    word: String,
    frequency: u64,
    only_text: bool,
}

fn count_words(v: &[Message]) -> Vec<SortedWord> {
    let hash = Arc::new(Mutex::new(HashMap::<String, u64>::new()));
    v.iter().for_each(|f| {
        let temp = f.contents.trim().replace("\n", " ");
        let words = temp.split(" ").into_iter();
        words.for_each(|word| {
            let word = word.trim().to_lowercase();
            let mut hash = hash.lock().unwrap();
            if let Some(key) = hash.get(&word).copied() {
                hash.insert(word.to_string(), key + 1);
            } else {
                hash.insert(word.to_string(), 1);
            };
        });
    });
    let mut h = hash.lock().unwrap();
    println!("Found {} individual words", h.len());
    let hashes = h.drain();
    let mut v = vec![];
    for (word, frequency) in hashes {
        let only_text = word.chars().all(|f| f.is_alphanumeric());
        v.push(SortedWord {
            word,
            frequency,
            only_text,
        });
    }
    v.sort_by(|a, b| b.frequency.cmp(&a.frequency));
    v
}

fn test_text(word: &str) -> bool {
    let only_text = word.chars().all(|f| f.is_alphanumeric());
    only_text
}
