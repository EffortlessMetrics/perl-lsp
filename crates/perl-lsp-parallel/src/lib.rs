//! Parallel worker-pool processing helpers for LSP workloads.
//!
//! This microcrate owns one responsibility: process file-like items in parallel
//! using a bounded worker pool and collect the resulting outputs.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

use std::sync::{Arc, Mutex, mpsc};
use std::thread;

/// Process files in parallel with a worker pool.
///
/// Distributes file processing across up to `num_workers` threads for faster indexing.
/// If `num_workers` is zero, processing falls back to single-threaded execution.
pub fn process_files_parallel<T, F>(files: Vec<String>, num_workers: usize, processor: F) -> Vec<T>
where
    T: Send + 'static,
    F: Fn(String) -> T + Send + Sync + 'static,
{
    if num_workers == 0 {
        return files.into_iter().map(processor).collect();
    }

    let (tx, rx) = mpsc::channel();
    let work_queue = Arc::new(Mutex::new(files));
    let processor = Arc::new(processor);

    let mut handles = Vec::new();

    for _ in 0..num_workers {
        let tx = tx.clone();
        let work_queue = Arc::clone(&work_queue);
        let processor = Arc::clone(&processor);

        let handle = thread::spawn(move || {
            loop {
                let file = {
                    let Ok(mut queue) = work_queue.lock() else {
                        break;
                    };
                    queue.pop()
                };

                match file {
                    Some(f) => {
                        let result = processor(f);
                        if tx.send(result).is_err() {
                            break;
                        }
                    }
                    None => break,
                }
            }
        });

        handles.push(handle);
    }

    drop(tx);

    for handle in handles {
        let _ = handle.join();
    }

    rx.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::process_files_parallel;

    #[test]
    fn processes_all_files_in_parallel() {
        let processed = Arc::new(AtomicUsize::new(0));
        let files = vec!["file1.pl".to_string(), "file2.pl".to_string(), "file3.pl".to_string()];

        let processed_clone = Arc::clone(&processed);
        let results = process_files_parallel(files, 2, move |_file| {
            processed_clone.fetch_add(1, Ordering::SeqCst);
            42
        });

        assert_eq!(results.len(), 3);
        assert_eq!(processed.load(Ordering::SeqCst), 3);
        assert!(results.iter().all(|value| *value == 42));
    }

    #[test]
    fn zero_workers_falls_back_to_single_threaded_processing() {
        let processed = Arc::new(AtomicUsize::new(0));
        let files = vec!["a.pl".to_string(), "b.pl".to_string()];

        let processed_clone = Arc::clone(&processed);
        let results = process_files_parallel(files, 0, move |_file| {
            processed_clone.fetch_add(1, Ordering::SeqCst);
            "ok"
        });

        assert_eq!(processed.load(Ordering::SeqCst), 2);
        assert_eq!(results, vec!["ok", "ok"]);
    }
}
