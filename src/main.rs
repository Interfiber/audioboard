#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate log;

use parking_lot::Mutex;
use playback_rs::Song;
use rdev::{listen, Event};
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

lazy_static! {
    /*
        Current audio we are playing
     */
    static ref AUDIO_COUNT: Mutex<usize> = Mutex::new(0);

    /*
        List of all of the audios
     */
    static ref AUDIOS: Mutex<Vec<String>> = Mutex::new(vec![]);

    static ref CURRENT_AUDIO: Mutex<String> = Mutex::new(String::from(""));

    static ref PLAYING_AUDIO: Mutex<bool> = Mutex::new(false);
}

struct Player {}

impl Player {
    fn play_audio(&self) {
        let audios = AUDIOS.lock().clone();
        let audio_count: usize = *AUDIO_COUNT.lock();
        let audio = &audios[audio_count];

        if Path::new("audio_lock").exists() {
            info!("Audio already playing, terminating");
            std::fs::remove_file("audio_lock").expect("Failed to remove audio lock");
            return;
        }

        if audio_count != audios.len() - 1 {
            *AUDIO_COUNT.lock() += 1;
        } else {
            *AUDIO_COUNT.lock() = 0;
        }

        info!("Playing audio: {}, with index: {}", audio, audio_count);

        if !Path::new(&audio).exists() {
            info!("File does not exist: {}", audio);
            return;
        }

        std::fs::File::create("audio_lock").unwrap();
        *CURRENT_AUDIO.lock() = audio.to_string();

        // Spawn thread

        std::thread::spawn(move || {
            let audio_file = CURRENT_AUDIO.lock().to_string();
            let player = playback_rs::Player::new(None).unwrap();

            let song = Song::from_file(&audio_file, None).unwrap();

            player.play_song_next(&song, None).unwrap();

            while player.has_current_song() {
                let player_is_playing = Path::new("audio_lock").exists();

                if player_is_playing {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                } else {
                    break;
                }
            }

            if Path::new("audio_lock").exists() {
                std::fs::remove_file("audio_lock").expect("Failed to remove audio lock");
            }

            info!("Audio stopped");
        });
    }

    fn key_callback(&self, event: Event) {
        // println!("My callback {:?}", event);
        match event.name {
            Some(string) => {
                if string == "/" {
                    self.play_audio();
                }
            }
            None => (),
        }
    }
}

fn main() {
    pretty_env_logger::init();

    info!("Loading audios.list...");
    let file = File::open("audios.list").expect("Failed to open audios.list");
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let current_line = line.unwrap();

        if current_line.chars().nth(0).unwrap() != '#' {
            debug!("Found file: {}", current_line);

            if !Path::new(&current_line).exists() {
                info!("Could not find file: {}", current_line);
                std::process::exit(-1);
            }

            AUDIOS.lock().push(current_line.to_string());
        }
    }

    let total = AUDIOS.lock().len();
    info!("Finished loading a total of {} audio files", total);

    info!("Registering ctrl-c handler...");
    ctrlc::set_handler(move || {
        if Path::new("audio_lock").exists() {
            std::fs::remove_file("audio_lock").expect("Failed to remove lock file");
        }
        std::process::exit(0);
    })
    .expect("Error setting Ctrl-C handler");

    let player = Player {};

    // This will block.
    if let Err(error) = listen(move |event| {
        player.key_callback(event);
    }) {
        error!("Error: {:?}", error)
    }
}
