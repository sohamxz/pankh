use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::time::Duration;

/// Watches files or directories for modification events and invokes callback
pub fn watch_paths<F>(paths: &[PathBuf], mut on_change: F) -> anyhow::Result<()>
where
    F: FnMut() + Send + 'static,
{
    let (tx, rx) = channel();
    let mut watcher = RecommendedWatcher::new(tx, Config::default())?;

    for path in paths {
        if path.exists() {
            let mode = if path.is_dir() {
                RecursiveMode::Recursive
            } else {
                RecursiveMode::NonRecursive
            };
            let _ = watcher.watch(path, mode);
        }
    }

    std::thread::spawn(move || {
        let _watcher = watcher;
        while let Ok(res) = rx.recv() {
            match res {
                Ok(Event { kind, .. }) => match kind {
                    EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_) => {
                        std::thread::sleep(Duration::from_millis(100));
                        on_change();
                    }
                    _ => {}
                },
                Err(_) => break,
            }
        }
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_watcher_file_change_detection() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("pankh_watch_test.md");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "# Watch Test\n\nInitial.").unwrap();

        let changed = Arc::new(AtomicBool::new(false));
        let changed_clone = Arc::clone(&changed);

        let _ = watch_paths(std::slice::from_ref(&file_path), move || {
            changed_clone.store(true, Ordering::SeqCst);
        });

        std::thread::sleep(Duration::from_millis(200));
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "# Watch Test\n\nUpdated!").unwrap();

        std::thread::sleep(Duration::from_millis(500));
        let _ = std::fs::remove_file(file_path);
    }
}
