//! A one-character spinner on standard error while fue searches, shown only on a terminal so piped output and scripts never see it.

use std::io::{IsTerminal, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// Braille dots, or ASCII on Windows, where common console fonts lack braille.
const FRAMES: &[char] = if cfg!(windows) {
    &['|', '/', '-', '\\']
} else {
    &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏']
};
const INTERVAL: Duration = Duration::from_millis(80);

/// Spins until dropped, then erases itself.
pub struct Spinner {
    running: Option<(Arc<AtomicBool>, JoinHandle<()>)>,
}

impl Spinner {
    /// Spins on standard error when it is a terminal.
    pub fn start() -> Self {
        if std::io::stderr().is_terminal() {
            Self::start_on(std::io::stderr())
        } else {
            Spinner { running: None }
        }
    }

    /// Spins on `output`.
    pub fn start_on(mut output: impl Write + Send + 'static) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = Arc::clone(&stop);
        let thread = thread::spawn(move || {
            for frame in FRAMES.iter().cycle() {
                if stopped.load(Ordering::Acquire) {
                    break;
                }
                let _ = write!(output, "\r{frame}");
                let _ = output.flush();
                thread::park_timeout(INTERVAL);
            }
            let _ = write!(output, "\r \r");
            let _ = output.flush();
        });
        Spinner {
            running: Some((stop, thread)),
        }
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        if let Some((stop, thread)) = self.running.take() {
            stop.store(true, Ordering::Release);
            thread.thread().unpark();
            let _ = thread.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Output shared with the spinner's thread.
    #[derive(Clone, Default)]
    struct Shared(Arc<Mutex<Vec<u8>>>);

    impl Write for Shared {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn spins_then_erases_itself() {
        let output = Shared::default();
        let spinner = Spinner::start_on(output.clone());
        thread::sleep(INTERVAL * 3);
        drop(spinner);
        let written = String::from_utf8(output.0.lock().unwrap().clone()).unwrap();
        assert!(
            written.starts_with(&format!("\r{}\r{}", FRAMES[0], FRAMES[1])),
            "{written:?}"
        );
        assert!(written.ends_with("\r \r"), "{written:?}");
    }

    #[test]
    fn stops_without_waiting_for_the_next_frame() {
        let spinner = Spinner::start_on(Shared::default());
        let started = std::time::Instant::now();
        drop(spinner);
        assert!(started.elapsed() < INTERVAL, "{:?}", started.elapsed());
    }
}
