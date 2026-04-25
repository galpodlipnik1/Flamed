use crate::error::{AppError, AppResult};
use rodio::{Decoder, OutputStream, Sink};
use std::{
    fs::File,
    io::{BufReader, Cursor},
    thread,
};
use tauri::{path::BaseDirectory, AppHandle, Manager};

pub fn play_death_sound(app: &AppHandle, volume: f32) -> AppResult<()> {
    let path = app
        .path()
        .resolve("resources/death_sound.mp3", BaseDirectory::Resource)
        .map_err(|error| AppError::Audio(error.to_string()))?;

    let volume = volume.clamp(0.0, 1.0);

    thread::spawn(move || {
        let Ok(file) = File::open(path) else {
            return;
        };

        let Ok(source) = Decoder::new(BufReader::new(file)) else {
            return;
        };

        let Ok((_stream, handle)) = OutputStream::try_default() else {
            return;
        };

        let Ok(sink) = Sink::try_new(&handle) else {
            return;
        };

        sink.set_volume(volume);
        sink.append(source);
        sink.sleep_until_end();
    });

    Ok(())
}

pub fn play_wav_bytes(bytes: Vec<u8>, volume: f32) -> AppResult<()> {
    let volume = volume.clamp(0.0, 1.0);

    thread::spawn(move || {
        let cursor = Cursor::new(bytes);
        let Ok(source) = Decoder::new(BufReader::new(cursor)) else {
            return;
        };

        let Ok((_stream, handle)) = OutputStream::try_default() else {
            return;
        };

        let Ok(sink) = Sink::try_new(&handle) else {
            return;
        };

        sink.set_volume(volume);
        sink.append(source);
        sink.sleep_until_end();
    });

    Ok(())
}
