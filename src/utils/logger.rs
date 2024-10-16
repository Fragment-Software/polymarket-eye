use tracing_appender::{
    non_blocking::WorkerGuard,
    rolling::{RollingFileAppender, Rotation},
};
use tracing_subscriber::{
    filter::LevelFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt, Layer,
};

const LOGS_FOLDER_PATH: &str = "data/logs";

fn init_logger(logs_folder_path: &str) -> WorkerGuard {
    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::HOURLY)
        .filename_prefix("app")
        .filename_suffix("log")
        .build(logs_folder_path)
        .expect("Appender to build");

    let (writer, guard) = tracing_appender::non_blocking(file_appender);

    let stdout_filter = LevelFilter::INFO;
    let file_filter = LevelFilter::INFO;

    let stdout_layer = fmt::layer()
        .with_writer(std::io::stdout)
        .with_thread_ids(true)
        .pretty() // comment this out if want to use the default format
        .with_ansi(true)
        .with_filter(stdout_filter);

    let file_layer = fmt::layer()
        .with_writer(writer)
        .with_ansi(false)
        .with_filter(file_filter);

    tracing_subscriber::registry()
        .with(stdout_layer)
        .with(file_layer)
        .init();

    guard
}

pub fn init_default_logger() -> WorkerGuard {
    init_logger(LOGS_FOLDER_PATH)
}
