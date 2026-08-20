use cosmox_configuration::Configuration;
use migration::MigratorTrait;
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    collections::HashMap,
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    path::Path,
    process::exit,
    sync::atomic::Ordering,
};
use sysinfo::System;
use validator::{Validate, ValidationErrorsKind};

use crate::get_db_connection;

#[derive(Serialize, Deserialize)]
pub struct SystemInfo {
    pub os: String,
    pub name: &'static str,
    pub version: &'static str,
    pub author: &'static str,
    pub is_first_boot: bool,
}

/// Errors related to system management operations (e.g., shutdown, restart).
#[derive(Debug, thiserror::Error)]
pub enum SystemError {
    #[error("System resource '{0}' not found.")]
    NotFound(String),

    #[error("Not authorized to perform system operation: {0}.")]
    Unauthorized(String),

    #[error("System is already in state '{0}'.")]
    AlreadyInState(String),

    #[error("System operation '{0}' failed: {1}")]
    OperationFailed(&'static str, String),

    #[error("Invalid system state for operation '{0}': current state is '{1}'.")]
    InvalidState(String, String),

    #[error("Validate failed")]
    Validation(HashMap<Cow<'static, str>, ValidationErrorsKind>),

    #[error("System configuration invalid: {0}")]
    ConfigurationError(String),

    #[error("System shutdown initiated.")]
    ShutdownInitiated, // Specific for async operations that might not immediately fail

    /// Indicates an unexpected server-side issue.
    #[error("Internal server error: {0}")]
    InternalError(String),
}

pub async fn info() -> Result<SystemInfo, SystemError> {
    // let sys = System::new_all();
    let info = SystemInfo {
        os: System::name().unwrap_or(String::from("Unknown")),
        name: env!("PROJECT_NAME"),
        version: env!("CARGO_PKG_VERSION"),
        author: env!("CARGO_PKG_AUTHORS"),
        is_first_boot: Configuration::get_global_configuration()
            .state
            .is_first_boot
            .load(Ordering::Relaxed),
    };
    Ok(info)
}

/// Restart server
pub async fn restart() -> Result<(), SystemError> {
    unimplemented!("Not implemented restart api")
}

pub async fn shutdown() -> ! {
    exit(0);
}

pub async fn about() -> Result<String, SystemError> {
    let mut readme = match File::open("./README.md") {
        Ok(file) => file,
        Err(err) => return Err(SystemError::OperationFailed("open", err.to_string())),
    };
    let metadata = match readme.metadata() {
        Ok(metadata) => metadata,
        Err(err) => return Err(SystemError::OperationFailed("metadata", err.to_string())),
    };
    let mut reuslt = String::with_capacity(metadata.len() as usize);
    match readme.read_to_string(&mut reuslt) {
        Ok(_) => Ok(reuslt),
        Err(err) => Err(SystemError::OperationFailed("read", err.to_string())),
    }
}

/// File name prefix for daily-rotated log files (must match `main.rs`).
const LOG_FILE_PREFIX: &str = "app.log";
/// Upper bound for a valid `limit` value; enforced by the
/// `#[validate(range)]` attribute on `LogQueryRequest.limit`.
const MAX_LIMIT: usize = 1000;
/// Upper bound of bytes read from the end of a log file per request.
const MAX_TAIL_BYTES: u64 = 8 * 1024 * 1024;

/// Log severity level; the variants serialize as uppercase (`WARN`) to match
/// the `LEVEL` token the fmt layer writes to the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum LogLevel {
    #[default]
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    fn from_token(token: &str) -> Option<Self> {
        match token {
            "TRACE" => Some(Self::Trace),
            "DEBUG" => Some(Self::Debug),
            "INFO" => Some(Self::Info),
            "WARN" => Some(Self::Warn),
            "ERROR" => Some(Self::Error),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Validate)]
pub struct LogQueryRequest {
    /// Log date in `YYYY-MM-DD` form (defaults to today). A full file name
    /// like `app.log.2026-08-18` is also accepted.
    pub file: Option<String>,
    /// Maximum number of log records to return. Omitted means no limit;
    /// must be in `1..=MAX_LIMIT`, anything else is rejected.
    #[validate(range(min = 1, max = MAX_LIMIT, message = "`limit` must be omitted for no limit, or be in 1..=1000"))]
    pub limit: Option<usize>,
    /// Only return records of this severity.
    pub level: Option<LogLevel>,
    /// Only return records containing this substring.
    pub keyword: Option<String>,
}

#[derive(Debug, Clone, Serialize, ::rkyv::Archive, ::rkyv::Serialize)]
#[rkyv(bytecheck())]
pub struct LogResponse {
    pub file: String,
    pub total: usize,
    pub logs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, ::rkyv::Archive, ::rkyv::Serialize)]
#[rkyv(bytecheck())]
pub struct LogFileInfo {
    pub name: String,
    pub date: String,
    pub size: u64,
}

/// Directory that holds the daily-rotated log files.
fn log_dir() -> &'static str {
    Configuration::get_global_configuration()
        .cosmox
        .log
        .path
        .as_str()
}

/// Logical name of today's log file: `app.log.{YYYY-MM-DD}` (UTC).
fn today_log_name() -> String {
    let date = chrono::Utc::now().format("%Y-%m-%d");
    format!("{}.{date}", LOG_FILE_PREFIX)
}

/// Resolve a `LogQueryRequest.file` (a date or full file name) to the logical
/// file name under `log_dir()`. The name round-trips: `LogResponse.file`
/// echoes it back and it is accepted again as `file`.
fn resolve_log_name(file: Option<&str>) -> String {
    match file {
        Some(name) if name.starts_with(LOG_FILE_PREFIX) => name.to_string(),
        Some(date) => format!("{}.{date}", LOG_FILE_PREFIX),
        None => today_log_name(),
    }
}

/// Read all physical lines within the trailing `MAX_TAIL_BYTES` window of
/// `path`. A leading partial line produced by the window start and the empty
/// tail line left by a trailing `\n` are discarded.
fn read_tail_lines(path: &str) -> Result<Vec<String>, SystemError> {
    let mut file = File::open(path)
        .map_err(|err| SystemError::OperationFailed("open log file", err.to_string()))?;
    let file_len = file
        .metadata()
        .map_err(|err| SystemError::OperationFailed("stat log file", err.to_string()))?
        .len();

    let start = file_len.saturating_sub(MAX_TAIL_BYTES);
    let mut file = match file.seek(SeekFrom::Start(start)) {
        Ok(_) => file,
        Err(err) => {
            return Err(SystemError::OperationFailed(
                "seek log file",
                err.to_string(),
            ));
        }
    };

    let mut buf = vec![0u8; (file_len - start) as usize];
    file.read_exact(&mut buf)
        .map_err(|err| SystemError::OperationFailed("read log file", err.to_string()))?;

    let text = String::from_utf8_lossy(&buf);
    // Drop the first, possibly partial, line when the window starts mid-line,
    // and the empty tail line left by a trailing `\n`.
    let lines: Vec<&str> = if start == 0 {
        text.trim_end_matches('\n').split('\n').collect()
    } else {
        text.split_once('\n')
            .map(|(_, rest)| rest)
            .unwrap_or("")
            .trim_end_matches('\n')
            .split('\n')
            .collect()
    };

    let lines = lines.iter().map(|line| line.to_string()).collect();
    Ok(lines)
}

/// Strip ANSI escape sequences (e.g. `\x1b[2m`) from a log line. The fmt
/// layers in `main.rs` write ANSI codes even to the file, so lines must be
/// cleaned before matching.
fn strip_ansi(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' && chars.next() == Some('[') {
            for esc in chars.by_ref() {
                if esc == 'm' {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Apply `level`/`keyword` filters to one whole log record.
///
/// `level` is the 2nd whitespace-separated token of the record (right-aligned
/// to 5 chars, e.g. ` WARN`); it is mapped to a `LogLevel` variant before
/// comparing. `keyword` is a plain substring match on the whole record
/// (including continuation lines). Returns the ANSI-stripped record when it
/// matches, `None` otherwise.
fn filter_log_record(
    record: &str,
    level: Option<LogLevel>,
    keyword: Option<&str>,
) -> Option<String> {
    let cleaned = strip_ansi(record);
    if let Some(level) = level {
        let record_level = cleaned
            .split_whitespace()
            .nth(1)
            .and_then(LogLevel::from_token);
        if record_level != Some(level) {
            return None;
        }
    }
    if let Some(keyword) = keyword
        && !cleaned.contains(keyword)
    {
        return None;
    }
    Some(cleaned)
}

/// Group physical lines into whole log records. A record starts at a line
/// whose 2nd whitespace-separated token is a known `LogLevel` (the fmt layer
/// writes `timestamp LEVEL module: message`); following lines belong to the
/// same record until the next record start. Multi-line messages (e.g. json
/// payloads) are kept intact. Orphan continuation lines at the window start
/// (the tail cut into a record) are dropped.
fn group_log_records(lines: Vec<String>) -> Vec<String> {
    let mut records: Vec<String> = Vec::new();
    for line in lines {
        let is_start = strip_ansi(&line)
            .split_whitespace()
            .nth(1)
            .and_then(LogLevel::from_token)
            .is_some();
        if is_start {
            records.push(line);
        } else if let Some(last) = records.last_mut() {
            last.push('\n');
            last.push_str(&line);
        }
    }
    records
}

/// Apply `level`/`keyword` filters to `records`, then keep the newest `limit`
/// matches. Returns `(total, logs)`: `total` counts every matching record in
/// the window (before truncation), `logs` holds the newest `limit` of them.
fn filter_and_truncate(
    records: Vec<String>,
    level: Option<LogLevel>,
    keyword: Option<&str>,
    limit: usize,
) -> (usize, Vec<String>) {
    let matched: Vec<String> = records
        .iter()
        .filter_map(|record| filter_log_record(record, level, keyword))
        .collect();
    let total = matched.len();
    let logs = matched
        .into_iter()
        .skip(total.saturating_sub(limit))
        .collect();
    (total, logs)
}

pub async fn get_log(query: LogQueryRequest) -> Result<LogResponse, SystemError> {
    query
        .validate()
        .map_err(|err| SystemError::Validation(err.errors().clone()))?;
    let limit = query.limit.unwrap_or(usize::MAX);
    let name = resolve_log_name(query.file.as_deref());
    let path = format!("{}/{}", log_dir(), name);

    if !Path::new(&path).exists() {
        return Err(SystemError::NotFound(name));
    }

    let tail = read_tail_lines(&path)?;
    let records = group_log_records(tail);
    let (total, logs) = filter_and_truncate(records, query.level, query.keyword.as_deref(), limit);

    Ok(LogResponse {
        file: name,
        total,
        logs,
    })
}

pub async fn log_files() -> Result<Vec<LogFileInfo>, SystemError> {
    let dir = log_dir();
    let entries = fs::read_dir(dir)
        .map_err(|err| SystemError::OperationFailed("read log dir", err.to_string()))?;

    let mut files = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let Some(date) = name.strip_prefix(&format!("{}.", LOG_FILE_PREFIX)) else {
            continue;
        };
        // Only accept names that look like a date suffix.
        if date.len() != 10 || !date.chars().all(|c| c == '-' || c.is_ascii_digit()) {
            continue;
        }
        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
        files.push(LogFileInfo {
            name: name.to_string(),
            date: date.to_string(),
            size,
        });
    }

    files.sort_by(|a, b| b.date.cmp(&a.date));
    Ok(files)
}

pub async fn delete_all() -> Result<(), SystemError> {
    let db = get_db_connection().await;
    migration::Migrator::fresh(db.as_ref())
        .await
        .inspect_err(|err| log::error!("Failed to reset database: {err}"))
        .map_err(|err| SystemError::OperationFailed("fresh", err.to_string()))?;

    if let Err(err) = std::fs::remove_file(".first_boot.lock") {
        log::error!("Failed to remove .first_boot.lock: {err}");
    }

    Configuration::get_global_configuration()
        .state
        .is_first_boot
        .store(true, Ordering::Relaxed);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_ansi_removes_escape_sequences() {
        let line = "\x1b[2m2026-08-19T06:16:20.229802Z\x1b[0m \x1b[34mDEBUG\x1b[0m msg";
        assert_eq!(strip_ansi(line), "2026-08-19T06:16:20.229802Z DEBUG msg");
    }

    #[test]
    fn strip_ansi_keeps_plain_lines_unchanged() {
        let line = "plain text without escapes";
        assert_eq!(strip_ansi(line), line);
    }

    #[test]
    fn filter_log_record_padded_level() {
        // WARN/INFO are right-aligned to 5 chars inside the ANSI color codes.
        let record = "\x1b[2m2026-08-19T06:16:20.232902Z\x1b[0m \x1b[33m WARN\x1b[0m \x1b[2mcosmox_backend_api::auth::access_check\x1b[0m\x1b[2m:\x1b[0m JWT verification failed";
        let matched = filter_log_record(record, Some(LogLevel::Warn), None).expect("match");
        // Two spaces after the timestamp: one after the ANSI reset code, one
        // from the padded ` WARN` token.
        assert_eq!(
            matched,
            "2026-08-19T06:16:20.232902Z  WARN cosmox_backend_api::auth::access_check: JWT verification failed"
        );
        assert!(filter_log_record(record, Some(LogLevel::Debug), None).is_none());
        assert!(filter_log_record(record, Some(LogLevel::Trace), None).is_none());
    }

    #[test]
    fn filter_log_record_keyword() {
        let record = "2026-08-19T06:16:20.229802Z DEBUG cosmox_scanner::worker: scan done";
        let matched = filter_log_record(record, None, Some("scan done")).expect("match");
        assert_eq!(matched, record);
        assert!(filter_log_record(record, None, Some("nonexistent")).is_none());
    }

    #[test]
    fn filter_log_record_unpadded_level() {
        let record = "2026-08-19T06:16:20.229802Z DEBUG cosmox_scanner::worker: scan done";
        assert!(filter_log_record(record, Some(LogLevel::Debug), None).is_some());
    }

    #[test]
    fn read_tail_lines_returns_all_window_lines() -> Result<(), SystemError> {
        let dir = std::env::temp_dir();
        let path = dir.join("cosmox_tail_test.log");
        std::fs::write(&path, "line1\nline2\nline3\nline4\nline5\n")
            .map_err(|e| SystemError::OperationFailed("write test file", e.to_string()))?;
        let tail = read_tail_lines(path.to_str().unwrap())?;
        std::fs::remove_file(&path).ok();
        assert_eq!(
            tail,
            vec![
                "line1".to_string(),
                "line2".to_string(),
                "line3".to_string(),
                "line4".to_string(),
                "line5".to_string()
            ]
        );
        Ok(())
    }

    #[test]
    fn group_log_records_merges_multiline() {
        let lines = vec![
            "2026-08-19T06:16:20.229802Z DEBUG cosmox_scanner::worker: {\"a\":1}".to_string(),
            "  \"b\":2".to_string(),
            "}".to_string(),
            "2026-08-19T06:16:20.230000Z  INFO other: done".to_string(),
        ];
        let records = group_log_records(lines);
        assert_eq!(records.len(), 2);
        assert_eq!(
            records[0],
            "2026-08-19T06:16:20.229802Z DEBUG cosmox_scanner::worker: {\"a\":1}\n  \"b\":2\n}"
        );
        assert_eq!(records[1], "2026-08-19T06:16:20.230000Z  INFO other: done");
    }

    #[test]
    fn group_log_records_drops_orphan_continuation() {
        // Tail window cut into a record: the leading continuation lines have
        // no record start to attach to and are dropped.
        let lines = vec![
            "  continuation without header".to_string(),
            "2026-08-19T06:16:20.229802Z  WARN real record".to_string(),
        ];
        let records = group_log_records(lines);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0], "2026-08-19T06:16:20.229802Z  WARN real record");
    }

    #[test]
    fn filter_log_record_matches_continuation_in_whole_record() {
        // The keyword appears in the 2nd physical line; the record must still
        // match as a whole (level taken from the 1st line).
        let record =
            "2026-08-19T06:16:20.229802Z DEBUG sqlx::query: summary=1\n  db.statement=\"SET mode\"";
        let matched =
            filter_log_record(record, Some(LogLevel::Debug), Some("SET mode")).expect("match");
        assert_eq!(matched, record);
        // Wrong level filters the whole record out, including its continuation.
        assert!(filter_log_record(record, Some(LogLevel::Warn), Some("SET mode")).is_none());
    }

    #[test]
    fn resolve_log_name_passthrough_and_date_expansion() {
        // A full file name passes through unchanged (round-trip value).
        assert_eq!(
            resolve_log_name(Some("app.log.2026-08-18")),
            "app.log.2026-08-18"
        );
        // A bare date is expanded to the prefixed file name.
        assert_eq!(resolve_log_name(Some("2026-08-18")), "app.log.2026-08-18");
        // No file means today; only the prefix + date shape is asserted.
        let today = resolve_log_name(None);
        assert!(today.starts_with("app.log.20"));
        assert_eq!(today.len(), "app.log.2026-08-18".len());
    }

    #[test]
    fn filter_and_truncate_total_counts_all_matches() {
        let records = vec![
            "2026-08-19T06:16:20.229802Z DEBUG a: one".to_string(),
            "2026-08-19T06:16:20.230000Z  INFO b: two".to_string(),
            "2026-08-19T06:16:20.231000Z DEBUG c: three".to_string(),
            "2026-08-19T06:16:20.232000Z  WARN d: four".to_string(),
        ];
        // Every record matches (no filters); limit 2 keeps the newest 2,
        // while `total` still reports all 4.
        let (total, logs) = filter_and_truncate(records, None, None, 2);
        assert_eq!(total, 4);
        assert_eq!(
            logs,
            vec![
                "2026-08-19T06:16:20.231000Z DEBUG c: three".to_string(),
                "2026-08-19T06:16:20.232000Z  WARN d: four".to_string(),
            ]
        );
    }

    #[test]
    fn filter_and_truncate_level_filters_before_counting() {
        let records = vec![
            "2026-08-19T06:16:20.229802Z DEBUG a: one".to_string(),
            "2026-08-19T06:16:20.230000Z  INFO b: two".to_string(),
            "2026-08-19T06:16:20.231000Z DEBUG c: three".to_string(),
        ];
        // Only DEBUG records count towards `total` and appear in `logs`.
        let (total, logs) = filter_and_truncate(records, Some(LogLevel::Debug), None, 10);
        assert_eq!(total, 2);
        assert_eq!(
            logs,
            vec![
                "2026-08-19T06:16:20.229802Z DEBUG a: one".to_string(),
                "2026-08-19T06:16:20.231000Z DEBUG c: three".to_string(),
            ]
        );
    }

    #[test]
    fn validate_limit() {
        // Omitted means no limit; values in 1..=MAX_LIMIT pass through.
        assert!(LogQueryRequest::default().validate().is_ok());
        assert!(
            LogQueryRequest {
                limit: Some(1),
                ..Default::default()
            }
            .validate()
            .is_ok()
        );
        assert!(
            LogQueryRequest {
                limit: Some(MAX_LIMIT),
                ..Default::default()
            }
            .validate()
            .is_ok()
        );
        // 0 and values above the cap are rejected.
        assert!(
            LogQueryRequest {
                limit: Some(0),
                ..Default::default()
            }
            .validate()
            .is_err()
        );
        assert!(
            LogQueryRequest {
                limit: Some(MAX_LIMIT + 1),
                ..Default::default()
            }
            .validate()
            .is_err()
        );
    }

    #[test]
    fn filter_and_truncate_unbounded_keeps_everything() {
        let records = vec![
            "2026-08-19T06:16:20.229802Z DEBUG a: one".to_string(),
            "2026-08-19T06:16:20.230000Z  INFO b: two".to_string(),
        ];
        let (total, logs) = filter_and_truncate(records, None, None, usize::MAX);
        assert_eq!(total, 2);
        assert_eq!(logs.len(), 2);
    }
}
