pub mod log {
  #[macro_export]
  macro_rules! error {
    ($($arg:tt)*) => {
      cosmox::plugin::cosmox_api::log(
        cosmox::plugin::cosmox_types::LogLevel::Error,
        &format!($($arg)*),
      );
    }
  }

  #[macro_export]
  macro_rules! warn {
    ($($arg:tt)*) => {
      cosmox::plugin::cosmox_api::log(
        cosmox::plugin::cosmox_types::LogLevel::Warn,
        &format!($($arg)*),
      )
    }
  }

  #[macro_export]
  macro_rules! info {
    ($($arg:tt)*) => {
      cosmox::plugin::cosmox_api::log(
        cosmox::plugin::cosmox_types::LogLevel::Info,
        &format!($($arg)*),
      );
    }
  }

  #[macro_export]
  macro_rules! debug {
    ($($arg:tt)*) => {
      cosmox::plugin::cosmox_api::log(
        cosmox::plugin::cosmox_types::LogLevel::Debug,
        &format!($($arg)*),
      );
    }
  }

  #[macro_export]
  macro_rules! trace {
    ($($arg:tt)*) => {
      cosmox::plugin::cosmox_api::log(
        cosmox::plugin::cosmox_types::LogLevel::Trace,
        &format!($($arg)*),
      );
    }
  }

  #[macro_export]
  macro_rules! fatal {
    ($($arg:tt)*) => {
      cosmox::plugin::cosmox_api::log(
        cosmox::plugin::cosmox_types::LogLevel::Fatal,
        &format!($($arg)*),
      );
    }
  }
}
