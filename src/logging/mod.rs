use chrono::Local;
use env_logger::{Builder, Env, fmt::style::Color};
use log::Level;
use std::io::Write;

pub fn init() {
    let env = Env::default().filter_or("RUST_LOG", "trace");
    let mut builder = Builder::from_env(env);

    builder.format(|buf, record| {
        let pkg_name = env!("CARGO_PKG_NAME").replace('-', "_");
        if !record.target().starts_with(&pkg_name) {
            return Ok(());
        }

        let ts = Local::now().format("%Y-%m-%d %H:%M:%S");

        // 正确方式：使用 default_level_style(...)
        let level_name = format!("{}", record.level());
        let colored_level = match record.level() {
            Level::Trace => format!("\x1b[35;1m[🐶 {}]\x1b[0m", level_name), // magenta
            Level::Debug => format!("\x1b[34;1m[🚗 {}]\x1b[0m", level_name), // blue
            Level::Info => format!("\x1b[32;1m[✅ {} ]\x1b[0m", level_name),  // green
            Level::Warn => format!("\x1b[33;1m[⚠️  {} ]\x1b[0m", level_name),  // yellow
            Level::Error => format!("\x1b[31;1m[❌ {}]\x1b[0m", level_name), // bold red
        };

        writeln!(
            buf,
            "[{}] - {} - {}:{}  {}",
            ts,
            colored_level,
            record.file().unwrap_or("unknown"),
            record.line().unwrap_or(0),
            record.args()
        )
    });

    builder.init();
}
