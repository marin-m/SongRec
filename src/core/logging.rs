
use log::Level;
use glib::{LogLevel, LogWriterOutput};
use std::collections::HashMap;

pub struct Logging;

impl Logging {
    pub fn setup_logging(glib_level: log::LevelFilter, songrec_level: log::LevelFilter) {
        // TODO: Improve the format?
        // + Bind to a file in addition to the standard output?

        fern::Dispatch::new()
            .format(|out, message, record| {
                out.finish(format_args!(
                    "[{} {} {} {}:{}] {}",
                    humantime::format_rfc3339_seconds(std::time::SystemTime::now()),
                    record.level(),
                    record.target(),
                    // record.module_path().unwrap_or("??"), <- Usually same as target
                    record.file().unwrap_or("??"),
                    match record.line() {
                        Some(line) => line.to_string(),
                        None => "??".to_string()
                    },
                    message
                ))
            })
            .level(glib_level)
            .level_for("songrec", songrec_level)
            .chain(std::io::stderr())
            .apply().unwrap();
    }

    pub fn bind_glib_logging() {
        // Handle structured GLib logging and route it to the `log` crate

        glib::log_set_writer_func(|level, log_fields| {

            // WIP:
            // Use: https://docs.rs/log/0.4.27/log/struct.Record.html
            // https://docs.rs/log/0.4.27/log/struct.RecordBuilder.html
            // https://docs.rs/log/0.4.27/log/fn.logger.html + https://docs.rs/log/0.4.27/log/trait.Log.html#tymethod.log
            
            let mut fields: HashMap<String, String> = HashMap::new();
            
            for field in log_fields {
                if let Some(value) = field.value_str() {
                    fields.insert(field.key().to_string(), value.to_string());
                }
            }

            let log_level = match level {
                LogLevel::Critical => Level::Error,
                LogLevel::Error => Level::Error,
                LogLevel::Warning => Level::Warn,
                LogLevel::Message => Level::Info,
                LogLevel::Info => Level::Info,
                LogLevel::Debug => Level::Debug
            };

            let message = fields.get("MESSAGE").map_or("??", |v| v);
            log::logger().log(
                &log::Record::builder()
                            .args(format_args!("{}", message))
                            .level(log_level)
                            .target(fields.get("GLIB_DOMAIN").map_or("Glib", |v| v))
                            .file(fields.get("CODE_FILE").map(|x| x.as_str()))
                            .line(fields.get("CODE_LINE").map(|x| x.parse::<u32>().unwrap_or(0)))
                            .module_path(fields.get("GLIB_DOMAIN").map(|x| x.as_str()))
                            .key_values(&fields)
                            .build()
            );

            LogWriterOutput::Handled
        });

        // Handle unstructured logging

        glib::log_set_default_handler(glib::rust_log_handler);
    }
}