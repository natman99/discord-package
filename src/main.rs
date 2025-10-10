use eframe::egui;
use egui::text::LayoutJob;
use egui::{ScrollArea, Ui};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rayon::*;
use serde::Deserialize;
use serde::Serialize;
use std::collections::hash_map::HashMap;
use std::fs;
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
    display_sentence: Vec<Message>,
    only_text: bool,
    search: String,
    processed_word_count: usize,
    proessed_messages_count: usize,
    shown_words_count: usize,
    state: State,
    prev_search_len: usize,
}
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum State{
    Sentence,
    #[default]
    Word,
}

impl MyEguiApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        let time = std::time::Instant::now();
        let m = read_messages();
        let end = time.elapsed().as_millis();
        println!("Processed {} messages in {} miliseconds", m.len(), end);
        let proessed_messages_count = m.len();
        let counted_words: Vec<SortedWord> = count_words(&m);
        let processed_word_count = counted_words.len();
        let display_sentence = m.clone();
        println!("Displaying {} rows", counted_words.len());
        let mut s: MyEguiApp = Self {
            counted_words,
            messages: m,
            display: Vec::new(),
            search: String::new(),
            only_text: false,
            processed_word_count,
            shown_words_count: 0,
            proessed_messages_count,
            state: Default::default(),
            display_sentence,
            prev_search_len: 0
        };
        
        let display: Vec<SortedWord> = s.counted_words.iter().map(|f| f.clone()).collect();
        let shown_words_count = display.len();
        s.display = display;
        s.shown_words_count = shown_words_count;
        s
        // Self::default()
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Discord messages!");
            let mut changed = false;
            let mut state_changed = false;

            ui.horizontal(|ui| {
                if ui.button("Word").clicked() {
                    self.state = State::Word;
                    state_changed = true
                }

                if ui.button("Sentence").clicked() {
                    self.state = State::Sentence;
                    state_changed = true
                }
            });

            ui.horizontal(|ui| {
                ui.label(format!(
                    "Processed messages: {}",
                    self.proessed_messages_count
                ));
                ui.label(format!("Processed words: {}", self.processed_word_count));
            });

            ui.label("Search:");
            let search_box = ui.text_edit_singleline(&mut self.search);
            changed = search_box.changed();
            if state_changed {
                changed = true
            }

            match self.state {
                State::Word => {
                    word_search_bar( self, ui, changed);
                },
                State::Sentence => {
                    sentence_search(self, ui, changed);
                }
            }

            
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
    let mut hash = HashMap::<String, u64>::new();
    v.iter().for_each(|f| {
        let temp = f.contents.trim().replace("\n", " ");
        let words = temp.split(" ").into_iter();
        words.for_each(|word| {
            let word = word.trim().to_lowercase();
            if let Some(key) = hash.get(&word).copied() {
                hash.insert(word.to_string(), key + 1);
            } else {
                hash.insert(word.to_string(), 1);
            };
        });
    });
    println!("Found {} individual words", hash.len());
    let mut v = vec![];
    for (word, frequency) in hash.drain() {
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

fn search_text<'a>(v: &'a Vec<SortedWord>, search_term: &str) -> Vec<&'a SortedWord> {
    let mut o: Vec<&'a SortedWord> = Vec::new();
    if test_text(search_term) {
        // ignore special characters search
        let d = v.iter().filter(|f| {
            f.word
                .chars()
                .filter(|f| f.is_alphanumeric())
                .collect::<String>()
                .contains(search_term.trim())
        });
        d.for_each(|f| o.push(f));
    } else {
        // exact search
        let d = v.iter().filter(|f|f.word.contains(search_term));
        d.for_each(|f|o.push(&f));
    }

    o

}



fn word_search_bar(app: &mut MyEguiApp, ui: &mut Ui, changed: bool) {
    ui.label(format!("Shown words: {}", app.display.iter().count()));

    ui.horizontal(|ui| {
                let only_text_check = ui.checkbox(&mut app.only_text, "Only allow alphanumeric");
                if changed || only_text_check.changed() {
                    if app.search.trim().is_empty() {
                    let all = app
                            .counted_words
                            .iter()
                            .filter(|f| {
                                if app.only_text == true {
                                    f.only_text == app.only_text
                                } else {
                                    true
                                }
                            })
                            .map(|f| f.clone())
                            .collect();
                        app.display = all;
                    } else {
                        app.display = search_text(&app.counted_words, &app.search)
                            .iter()
                            .filter(|f| {
                                if app.only_text == true {
                                    &f.only_text == &app.only_text
                                } else {
                                    true
                                }
                            })
                            .map(|f| *f)
                            .map(|f|f.clone())
                            .collect();
                    }
                }
            });
            let text_style = egui::TextStyle::Body;
            let row_height = ui.text_style_height(&text_style);
            ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height(ui.available_height())
                .show_rows(ui, row_height, app.display.len(), |ui, row_range| {
                    ui.set_min_height(ui.available_height());

                    for row in row_range {
                        let f = &app.display[row];
                        ui.horizontal(|ui| {
                            ui.label(format!("{}", f.word));
                            ui.separator();
                            ui.label(format!("{}", f.frequency));
                        });
                        ui.separator();
                    }
                });
}

fn sentence_search(app: &mut MyEguiApp, ui: &mut Ui, changed: bool) {
    ui.label(format!("Shown messages: {}", app.display_sentence.iter().count()));

    ui.horizontal(|ui| {
                
                if changed {
                    if app.search.trim().is_empty() {
                    let all = app.messages.clone();
                        app.display_sentence = all;
                    } else {
                        let search_me: &Vec<Message>;
                        if app.search.len() > app.prev_search_len {
                            search_me = &app.display_sentence
                        } else {
                            search_me = &app.messages
                        }
                        app.prev_search_len = app.search.len();

                        app.display_sentence = search_sentnece(&search_me, &app.search)
                            .iter()
    
                            .map(|f| *f)
                            .map(|f|f.clone())
                            .collect();
                    }
                }
            });
            let text_style = egui::TextStyle::Body;
            let row_height = ui.text_style_height(&text_style);
            ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height(ui.available_height())
                .show_rows(ui, row_height, app.display_sentence.len(), |ui, row_range| {
                    ui.set_min_height(ui.available_height());

                    for row in row_range {
                        let f = &app.display_sentence[row];
                        ui.horizontal(|ui| {
                            let text = egui::text::TextWrapping {
                                max_width: ui.available_width(),
                                max_rows: 4,
                                break_anywhere: true,
                                overflow_character: None,
                            };


                            ui.label(format!("{}", f.contents));
                            ui.separator();
                        });
                        ui.separator();
                    }
                });
}

fn search_sentnece<'a>(v: &'a Vec<Message>, search_term: &str) -> Vec<&'a Message> {
    let mut o: Vec<&'a Message> = Vec::new();
    if test_text(search_term) {
        // ignore special characters search
        let d = v.iter().filter(|f| {
            f.contents
                .chars()
                .filter(|f| f.is_alphanumeric())
                .collect::<String>()
                .to_lowercase()
                .contains(&search_term.trim().to_lowercase())
        });
        d.for_each(|f| o.push(f));
    } else {
        // exact search
        let d = v.iter().filter(|f|f.contents.to_lowercase().contains(&search_term.to_lowercase()));
        d.for_each(|f|o.push(&f));
    }
    o

}