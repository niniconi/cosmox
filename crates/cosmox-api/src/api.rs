use std::{fmt::Write, panic};

use anyhow::Result;
use log::{LevelFilter, SetLoggerError};

pub mod bindings {
    wit_bindgen::generate!({
        path: "wit",
        // the name of the world in the `*.wit` input file
        world: "plugin-host-world",
        pub_export_macro: true,
        default_bindings_module: "cosmox_api::api::bindings"
    });
}

pub struct Cosmox;
static COSMOX: Cosmox = Cosmox;

impl log::Log for Cosmox {
    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let level = match record.level() {
                log::Level::Error => bindings::cosmox::plugin::cosmox_types::LogLevel::Error,
                log::Level::Warn => bindings::cosmox::plugin::cosmox_types::LogLevel::Warn,
                log::Level::Debug => bindings::cosmox::plugin::cosmox_types::LogLevel::Debug,
                log::Level::Info => bindings::cosmox::plugin::cosmox_types::LogLevel::Info,
                log::Level::Trace => bindings::cosmox::plugin::cosmox_types::LogLevel::Trace,
            };

            let args = record.args();

            if let Some(message) = args.as_str() {
                bindings::cosmox::plugin::cosmox_api::log(level, message);
            } else {
                let mut message = String::with_capacity(128);

                write!(&mut message, "{}", args).unwrap();

                bindings::cosmox::plugin::cosmox_api::log(level, message.as_str());
            }
        }
    }

    fn flush(&self) {}

    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }
}

impl Cosmox {
    pub fn set_logger(level: LevelFilter) -> Result<(), SetLoggerError> {
        log::set_logger(&COSMOX).map(|()| log::set_max_level(level))
    }

    pub fn init() -> Result<()> {
        Self::set_logger(LevelFilter::Info)?;

        panic::set_hook(Box::new(|info| {
            let msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = info.payload().downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown panic".to_string()
            };

            let location = info
                .location()
                .map(|l| format!(" at {}:{}:{}", l.file(), l.line(), l.column()))
                .unwrap_or_default();

            bindings::cosmox::plugin::cosmox_api::log(
                bindings::cosmox::plugin::cosmox_types::LogLevel::Fatal,
                format!("{} {}", msg.as_str(), location).as_str(),
            );
        }));

        Ok(())
    }
}
