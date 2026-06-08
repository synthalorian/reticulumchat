use backtrace::Backtrace;
use chrono::Utc;
use std::panic::PanicHookInfo;
use std::path::PathBuf;

/// A captured crash report with backtrace and system information
#[derive(Debug, Clone)]
pub struct CrashReport {
    pub timestamp: String,
    pub payload: String,
    pub location: Option<String>,
    pub backtrace: String,
    pub version: String,
}

impl CrashReport {
    /// Create a new crash report from a panic hook info
    pub fn from_panic(info: &PanicHookInfo) -> Self {
        let backtrace = Backtrace::new();
        let backtrace_str = format!("{:?}", backtrace);

        let location = info.location().map(|loc| {
            format!("{}:{}:{}", loc.file(), loc.line(), loc.column())
        });

        let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic payload".to_string()
        };

        Self {
            timestamp: Utc::now().to_rfc3339(),
            payload,
            location,
            backtrace: backtrace_str,
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Format the crash report as a human-readable string
    pub fn format(&self) -> String {
        let mut output = String::new();
        output.push_str("========================================\n");
        output.push_str("  ReticulumChat Crash Report\n");
        output.push_str("========================================\n\n");
        output.push_str(&format!("Version:    {}\n", self.version));
        output.push_str(&format!("Timestamp:  {}\n", self.timestamp));
        output.push_str(&format!("\nPanic Message:\n  {}\n", self.payload));

        if let Some(ref loc) = self.location {
            output.push_str(&format!("\nLocation:\n  {}\n", loc));
        }

        output.push_str("\nBacktrace:\n");
        output.push_str(&self.backtrace);
        output.push('\n');

        output
    }

    /// Get the default crash reports directory path
    pub fn default_reports_dir() -> PathBuf {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home)
            .join(".reticulumchat")
            .join("crashes")
    }

    /// Save the crash report to the default reports directory
    pub fn save(&self) -> std::io::Result<PathBuf> {
        let dir = Self::default_reports_dir();
        std::fs::create_dir_all(&dir)?;

        let filename = format!(
            "crash-report-{}.txt",
            Utc::now().format("%Y%m%d-%H%M%S")
        );
        let path = dir.join(&filename);
        std::fs::write(&path, self.format())?;

        Ok(path)
    }
}

/// Install a panic hook that captures backtraces and writes crash reports
///
/// After installing the hook, any panic will:
/// 1. Capture a full backtrace
/// 2. Log the panic via tracing::error!
/// 3. Write a crash report file to ~/.reticulumchat/crashes/
/// 4. Print a message to stderr about the crash report location
pub fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let report = CrashReport::from_panic(info);

        // Log the panic via tracing
        tracing::error!("Application panicked: {}", report.payload);
        if let Some(ref loc) = report.location {
            tracing::error!("Panic location: {}", loc);
        }

        // Try to save the crash report
        match report.save() {
            Ok(path) => {
                eprintln!("\n[CRASH] The application has crashed.");
                eprintln!("[CRASH] A crash report has been saved to: {}", path.display());
                eprintln!("[CRASH] Please report this issue at: https://github.com/reticulumchat/reticulumchat/issues\n");
            }
            Err(e) => {
                eprintln!("\n[CRASH] The application has crashed.");
                eprintln!("[CRASH] Failed to save crash report: {}", e);
                eprintln!("[CRASH] Crash details:\n{}\n", report.format());
            }
        }
    }));
}

/// Install the panic hook and log that it's active
pub fn init() {
    install_panic_hook();
    tracing::debug!("Crash reporter panic hook installed");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crash_report_format() {
        let report = CrashReport {
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            payload: "test panic".to_string(),
            location: Some("src/main.rs:10:5".to_string()),
            backtrace: "backtrace lines".to_string(),
            version: "0.8.0".to_string(),
        };

        let formatted = report.format();
        assert!(formatted.contains("ReticulumChat Crash Report"));
        assert!(formatted.contains("test panic"));
        assert!(formatted.contains("src/main.rs:10:5"));
        assert!(formatted.contains("0.8.0"));
    }

    #[test]
    fn test_default_reports_dir() {
        let dir = CrashReport::default_reports_dir();
        assert!(dir.to_string_lossy().contains(".reticulumchat"));
        assert!(dir.to_string_lossy().contains("crashes"));
    }
}
