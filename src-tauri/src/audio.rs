use crate::error::{AppError, AppResult};
use rodio::{Decoder, OutputStream, Sink};
use std::{
    fs::File,
    io::{BufReader, Cursor},
    path::PathBuf,
    thread,
};
use tauri::{path::BaseDirectory, AppHandle, Manager};

fn play_file(path: PathBuf, volume: f32) {
    thread::spawn(move || {
        let Ok(file) = File::open(path) else { return };
        let Ok(source) = Decoder::new(BufReader::new(file)) else { return };
        let Ok((_stream, handle)) = OutputStream::try_default() else { return };
        let Ok(sink) = Sink::try_new(&handle) else { return };
        sink.set_volume(volume);
        sink.append(source);
        sink.sleep_until_end();
    });
}

fn play_resource(app: &AppHandle, name: &str, volume: f32) -> AppResult<()> {
    let path = app
        .path()
        .resolve(format!("resources/{name}"), BaseDirectory::Resource)
        .map_err(|e| AppError::Audio(e.to_string()))?;
    play_file(path, volume.clamp(0.0, 1.0));
    Ok(())
}

pub fn play_death_sound(app: &AppHandle, volume: f32) -> AppResult<()> {
    play_resource(app, "death_sound.mp3", volume)
}

pub fn play_win_sound(app: &AppHandle, volume: f32) -> AppResult<()> {
    play_resource(app, "win_sound.mp3", volume)
}

pub fn play_lose_sound(app: &AppHandle, volume: f32) -> AppResult<()> {
    play_resource(app, "lose_sound.mp3", volume)
}

pub fn play_wav_bytes(bytes: Vec<u8>, volume: f32) -> AppResult<()> {
    let volume = volume.clamp(0.0, 1.0);
    thread::spawn(move || {
        let cursor = Cursor::new(bytes);
        let Ok(source) = Decoder::new(BufReader::new(cursor)) else { return };
        let Ok((_stream, handle)) = OutputStream::try_default() else { return };
        let Ok(sink) = Sink::try_new(&handle) else { return };
        sink.set_volume(volume);
        sink.append(source);
        sink.sleep_until_end();
    });
    Ok(())
}
