use std::fs;
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

pub async fn init_logging(file_name: &str) {
    // Build logs/<YYYY-MM-DD>/ directory under current working directory (Beijing time, UTC+8)
    let beijing_offset = time::UtcOffset::from_hms(8, 0, 0).expect("valid UTC+8 offset");
    let today = time::OffsetDateTime::now_utc()
        .to_offset(beijing_offset)
        .date();
    let date_str = today
        .format(
            &time::format_description::parse("[year]-[month]-[day]").expect("valid date format"),
        )
        .expect("format date");

    let mut log_dir: PathBuf = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    log_dir.push("logs");
    log_dir.push(&date_str);
    fs::create_dir_all(&log_dir).expect("Failed to create log directory");

    // Create a non-blocking file writer to logs/<date>/<file_name>.log
    let file_appender = tracing_appender::rolling::never(&log_dir, file_name);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // Keep the guard alive inside a Tokio task and drop it on Ctrl-C for clean flush
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        drop(guard);
    });

    let beijing_offset = time::UtcOffset::from_hms(8, 0, 0).expect("valid UTC+8 offset");
    let time_format = time::format_description::parse(
        "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:6]",
    )
    .expect("valid time format");
    let timer = tracing_subscriber::fmt::time::OffsetTime::new(beijing_offset, time_format);

    // Create a layer that writes to the file
    let file_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_timer(timer.clone())
        .compact()
        .with_file(true)
        .with_line_number(true)
        .with_target(false)
        .with_writer(non_blocking);

    // Create a layer that writes to stdout (terminal)
    let stdout_layer = tracing_subscriber::fmt::layer()
        .with_ansi(true) // Enable colors for terminal
        .with_timer(timer)
        .compact()
        .with_file(true)
        .with_line_number(true)
        .with_target(false)
        .with_writer(std::io::stdout);

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    // Combine both layers with the filter
    use tracing_subscriber::layer::SubscriberExt;
    let subscriber = tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer)
        .with(stdout_layer);

    tracing::subscriber::set_global_default(subscriber).expect("Failed to set subscriber");
}
