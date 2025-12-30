use chrono::Local;
use env_logger::{Builder, Env};
use log::Level;
use serde::Serialize;
use std::collections::VecDeque;
use std::io::Write;
use std::sync::Mutex;

#[derive(Clone, Serialize, Debug)]
pub struct LogEntry {
    pub ts: String,
    pub level: String,
    pub target: String,
    pub message: String,
}

static LOG_BUFFER: Mutex<VecDeque<LogEntry>> = Mutex::new(VecDeque::new());
const MAX_LOGS: usize = 1000;

pub fn get_recent_logs() -> Vec<LogEntry> {
    match LOG_BUFFER.lock() {
        Ok(guard) => guard.iter().cloned().collect(),
        Err(_) => vec![],
    }
}

pub fn init() {
    let env = Env::default().filter_or("RUST_LOG", "trace");
    let mut builder = Builder::from_env(env);

    builder.format(|buf, record| {
        let pkg_name = env!("CARGO_PKG_NAME").replace('-', "_");
        if !record.target().starts_with(&pkg_name) {
            return Ok(());
        }

        let ts = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let level_string = record.level().to_string();
        let target = record.target().to_string();
        let message = format!("{}", record.args());

        // Push to buffer
        if let Ok(mut buffer) = LOG_BUFFER.lock() {
            if buffer.len() >= MAX_LOGS {
                buffer.pop_front();
            }
            buffer.push_back(LogEntry {
                ts: ts.clone(),
                level: level_string.clone(),
                target,
                message: message.clone(),
            });
        }

        // Console output
        let colored_level = match record.level() {
            Level::Trace => format!("\x1b[35;1m[🐶 {}]\x1b[0m", level_string), // magenta
            Level::Debug => format!("\x1b[34;1m[🚗 {}]\x1b[0m", level_string), // blue
            Level::Info => format!("\x1b[32;1m[✅ {} ]\x1b[0m", level_string), // green
            Level::Warn => format!("\x1b[33;1m[⚠️  {} ]\x1b[0m", level_string), // yellow
            Level::Error => format!("\x1b[31;1m[❌ {}]\x1b[0m", level_string), // bold red
        };

        writeln!(
            buf,
            "[{}] - {} - {}:{}  {}",
            ts,
            colored_level,
            record.file().unwrap_or("unknown"),
            record.line().unwrap_or(0),
            message
        )
    });

    builder.init();
}
