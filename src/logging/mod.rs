// src/logging/mod.rs

use log::Record;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub type LogBuffer = Arc<Mutex<Vec<(Instant, String)>>>;

pub fn initialize_logging(log_buffer: LogBuffer) -> Result<(), fern::InitError> {
    fs::create_dir_all("logs")?;

    let buffer_config = fern::Dispatch::new()
        .level(log::LevelFilter::Info)
        .chain(fern::Output::call(move |record: &Record| {
            let msg = format!(
                "[{}] [{}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.target(),
                record.args()
            );
            let mut buffer = log_buffer.lock().unwrap();
            buffer.push((Instant::now(), msg));
            if buffer.len() > 500 {
                buffer.remove(0);
            }
        }));

    let file_config = fern::Dispatch::new()
        .level(log::LevelFilter::Info)
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}][{}][{}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.target(),
                record.level(),
                message
            ))
        })
        .chain(fern::DateBased::new("logs/", "APOLLO-%Y-%m-%d.log"));

    fern::Dispatch::new()
        .chain(buffer_config)
        .chain(file_config)
        .apply()?;

    Ok(())
}