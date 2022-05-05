use colored::*;

static LOGGER: Logger = Logger;

pub fn init() {
  log::set_logger(&LOGGER).unwrap();
  log::set_max_level(log::LevelFilter::Trace);
}

struct Logger;

impl log::Log for Logger {
  fn enabled(&self, metadata: &log::Metadata) -> bool {
    metadata.target().starts_with(env!("CARGO_PKG_NAME"))
  }

  fn log(&self, record: &log::Record) {
    if self.enabled(record.metadata()) {
      let msg = format!("{}", record.args());

      match record.level() {
        log::Level::Error => println!("{}", msg.red()),
        log::Level::Warn => println!("{}", msg.yellow()),
        log::Level::Info => println!("{}", msg),
        log::Level::Debug => println!("{}", msg.white().dimmed()),
        log::Level::Trace => println!("{}", msg.white().dimmed()),
      };
    }
  }

  fn flush(&self) {}
}
