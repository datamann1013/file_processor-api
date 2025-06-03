use crate::error_handler::FileWriter;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use std::fs::{metadata, rename};
use std::io::{self};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// SafeFileWriter wraps a FileWriter and adds rate limiting, file rotation, and SMTP alerting
pub struct SafeFileWriter<F: FileWriter + 'static> {
    inner: Arc<F>,
    jsonl_path: PathBuf,
    temp_path: PathBuf,
    max_file_size: u64,
    rate_limit: usize, // max writes per interval
    interval: Duration,
    write_count: AtomicUsize,
    last_reset: Mutex<Instant>,
    consecutive_failures: AtomicUsize,
    alert_threshold: usize,
    smtp_config: Option<SmtpConfig>,
}

pub struct SmtpConfig {
    pub smtp_server: String,
    pub smtp_user: String,
    pub smtp_pass: String,
    pub to: String,
    pub from: String,
}

impl<F: FileWriter + 'static> SafeFileWriter<F> {
    pub fn new(
        inner: F,
        jsonl_path: PathBuf,
        temp_path: PathBuf,
        max_file_size: u64,
        rate_limit: usize,
        interval: Duration,
        alert_threshold: usize,
        smtp_config: Option<SmtpConfig>,
    ) -> Self {
        Self {
            inner: Arc::new(inner),
            jsonl_path,
            temp_path,
            max_file_size,
            rate_limit,
            interval,
            write_count: AtomicUsize::new(0),
            last_reset: Mutex::new(Instant::now()),
            consecutive_failures: AtomicUsize::new(0),
            alert_threshold,
            smtp_config,
        }
    }

    async fn check_and_rotate(&self, path: &PathBuf) -> io::Result<()> {
        if let Ok(meta) = metadata(path) {
            if meta.len() > self.max_file_size {
                let rotated =
                    path.with_extension(format!("rotated_{}", chrono::Utc::now().timestamp()));
                rename(path, rotated)?;
            }
        }
        Ok(())
    }

    async fn rate_limit(&self) -> bool {
        let mut last = self.last_reset.lock().await;
        if last.elapsed() > self.interval {
            self.write_count.store(0, Ordering::Relaxed);
            *last = Instant::now();
        }
        let count = self.write_count.fetch_add(1, Ordering::Relaxed);
        count < self.rate_limit
    }

    fn send_alert(&self, subject: &str, body: &str) {
        if let Some(cfg) = &self.smtp_config {
            let email = Message::builder()
                .from(cfg.from.parse().unwrap())
                .to(cfg.to.parse().unwrap())
                .subject(subject)
                .body(body.to_string())
                .unwrap();
            let creds = Credentials::new(cfg.smtp_user.clone(), cfg.smtp_pass.clone());
            let mailer = SmtpTransport::relay(&cfg.smtp_server)
                .unwrap()
                .credentials(creds)
                .build();
            let _ = mailer.send(&email); // ignore errors for now
        }
    }
}

#[async_trait::async_trait]
impl<F: FileWriter + 'static> FileWriter for SafeFileWriter<F> {
    async fn write_jsonl(&self, line: &str) -> io::Result<()> {
        self.check_and_rotate(&self.jsonl_path).await?;
        if !self.rate_limit().await {
            return Err(io::Error::new(io::ErrorKind::Other, "Rate limit exceeded"));
        }
        match self.inner.write_jsonl(line).await {
            Ok(()) => {
                self.consecutive_failures.store(0, Ordering::Relaxed);
                Ok(())
            }
            Err(e) => {
                let fails = self.consecutive_failures.fetch_add(1, Ordering::Relaxed) + 1;
                if fails >= self.alert_threshold {
                    self.send_alert(
                        "Persistent JSONL Write Failure",
                        &format!("{} failures: {}", fails, e),
                    );
                }
                Err(e)
            }
        }
    }
    async fn write_temp(&self, line: &str) -> io::Result<()> {
        self.check_and_rotate(&self.temp_path).await?;
        if !self.rate_limit().await {
            return Err(io::Error::new(io::ErrorKind::Other, "Rate limit exceeded"));
        }
        match self.inner.write_temp(line).await {
            Ok(()) => {
                self.consecutive_failures.store(0, Ordering::Relaxed);
                Ok(())
            }
            Err(e) => {
                let fails = self.consecutive_failures.fetch_add(1, Ordering::Relaxed) + 1;
                if fails >= self.alert_threshold {
                    self.send_alert(
                        "Persistent Temp Write Failure",
                        &format!("{} failures: {}", fails, e),
                    );
                }
                Err(e)
            }
        }
    }
}
