//! Parallel file-processing helpers for workspace operations.
//!
//! This crate has one narrow responsibility: run independent file tasks across
//! a fixed-size thread pool and collect results.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

use std::sync::{Arc, Mutex, mpsc};
use std::thread;

/// Process files in parallel with a worker pool.
///
/// Each worker repeatedly pops one file from the shared queue, runs
/// `processor`, and sends the result through a channel. The function returns
/// all results that were successfully produced.
#[must_use]
pub fn process_files_parallel<T, F>(files: Vec<String>, num_workers: usize, processor: F) -> Vec<T>
where
    T: Send + 'static,
    F: Fn(String) -> T + Send + Sync + 'static,
{
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
    use super::process_files_parallel;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn processes_all_files() {
        let files = vec!["one.pl".to_string(), "two.pl".to_string(), "three.pl".to_string()];
        let counter = Arc::new(AtomicUsize::new(0));

        let counter_clone = Arc::clone(&counter);
        let results = process_files_parallel(files, 2, move |_file| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            7
        });

        assert_eq!(counter.load(Ordering::SeqCst), 3);
        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|value| *value == 7));
    }

    #[test]
    fn returns_empty_when_no_files() {
        let results = process_files_parallel(Vec::new(), 4, |_file| 1usize);
        assert!(results.is_empty());
    }
}
