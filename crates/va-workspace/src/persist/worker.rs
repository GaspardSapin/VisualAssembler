//! The `va-io` worker thread (m4-editor-shell.md §4.7, architecture §4 T1, T2, T5, T6).
//!
//! Every file job of the shell runs here, one at a time and in submission order. A job sends
//! its own result (through a channel it captured) and calls the [`Waker`] so the UI thread
//! polls it on the next frame. A job that panics is caught (T6) and the next one runs.

use std::panic::{self, AssertUnwindSafe};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, PoisonError, RwLock};
use std::thread;
use std::time::Duration;

/// Wakes the UI thread when a worker result is ready (architecture T2).
pub type Waker = Arc<dyn Fn() + Send + Sync>;

/// One queued job.
type Job = Box<dyn FnOnce() + Send>;

/// The waker slot every handle and every [`IoWorker::waker`] closure shares.
type WakerSlot = Arc<RwLock<Option<Waker>>>;

/// The name of the worker thread (architecture §4).
const THREAD_NAME: &str = "va-io";

/// Handle of the `va-io` worker thread; `Clone` gives another handle to the same worker.
///
/// The thread ends once every handle is dropped and the jobs queued before that have run.
#[derive(Clone)]
pub struct IoWorker {
    /// The state every handle shares.
    shared: Arc<Shared>,
}

/// What the handles of one worker share.
struct Shared {
    /// The job queue of the thread (`Sender` is `Sync`, so handles can be used from any thread).
    jobs: Sender<Job>,
    /// The waker that jobs call; kept apart from `jobs` so a waker held by a result receiver
    /// never keeps the thread alive.
    waker: WakerSlot,
}

impl IoWorker {
    /// Starts the `va-io` thread.
    ///
    /// # Errors
    /// The error of `std::thread::Builder::spawn`.
    pub fn spawn() -> std::io::Result<Self> {
        let (jobs, queue) = mpsc::channel::<Job>();
        thread::Builder::new()
            .name(THREAD_NAME.to_owned())
            .spawn(move || run(&queue))?;
        Ok(Self {
            shared: Arc::new(Shared {
                jobs,
                waker: Arc::new(RwLock::new(None)),
            }),
        })
    }

    /// Stores the waker that jobs call when their result is ready; it replaces the previous one.
    pub fn set_waker(&self, waker: Waker) {
        *self
            .shared
            .waker
            .write()
            .unwrap_or_else(PoisonError::into_inner) = Some(waker);
    }

    /// A waker that calls the one in the slot (a no-op until [`IoWorker::set_waker`]); a waker
    /// taken before `set_waker` calls the waker set later.
    pub fn waker(&self) -> Waker {
        let slot = Arc::clone(&self.shared.waker);
        Arc::new(move || {
            // Clone the waker out of the slot so it never runs under the lock.
            let current = slot.read().unwrap_or_else(PoisonError::into_inner).clone();
            if let Some(waker) = current {
                waker();
            }
        })
    }

    /// Queues `job`; jobs run in order, and each one sends its own result.
    ///
    /// A job submitted after the thread is gone (it never stops while a handle exists) is
    /// dropped, which closes the result channel it captured.
    pub fn submit(&self, job: Box<dyn FnOnce() + Send>) {
        // The thread owns the receiver for as long as any handle exists, so a send only fails
        // if it died; dropping the job then disconnects its result channel.
        let _ = self.shared.jobs.send(job);
    }

    /// Waits until the jobs queued so far ran, at most `timeout`; false on timeout. Used at
    /// shutdown only (the one place the UI thread waits, m4 §4.8).
    pub fn flush(&self, timeout: Duration) -> bool {
        let (done, finished) = mpsc::channel::<()>();
        let marker: Job = Box::new(move || {
            let _ = done.send(());
        });
        if self.shared.jobs.send(marker).is_err() {
            return false;
        }
        finished.recv_timeout(timeout).is_ok()
    }
}

/// The loop of the `va-io` thread: runs each job under `catch_unwind` until every handle is
/// dropped (T6: a panicking job never stops the worker).
fn run(queue: &Receiver<Job>) {
    while let Ok(job) = queue.recv() {
        // The panic message was already printed by the panic hook; the job's result channel is
        // dropped with it, so its receiver sees a disconnection.
        let _ = panic::catch_unwind(AssertUnwindSafe(job));
    }
}
