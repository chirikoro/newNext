//! Background Job Queue for Hayabusa.
//!
//! In-memory async task queue with priorities, retries, scheduling,
//! and concurrency control. For persistent queues, use with Redis/Postgres.
//!
//! ```ignore
//! use hayabusa_core::prelude::*;
//!
//! let queue = JobQueue::new(4); // 4 concurrent workers
//! queue.enqueue(Job::new("send_email", r#"{"to":"user@example.com"}"#));
//! ```

use std::collections::VecDeque;
use std::sync::Arc;
use dashmap::DashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{Mutex, Notify};

// ─── Job ────────────────────────────────────────────────────

/// A background job
#[derive(Debug, Clone)]
pub struct Job {
    pub id: String,
    pub name: String,
    pub payload: String,
    pub priority: JobPriority,
    pub max_retries: u32,
    pub retry_count: u32,
    pub status: JobStatus,
    pub created_at: u64,
    pub scheduled_at: Option<u64>,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
    pub error: Option<String>,
    pub retry_delay_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum JobPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Retrying,
    Scheduled,
}

impl Job {
    pub fn new(name: impl Into<String>, payload: impl Into<String>) -> Self {
        let now = now_ms();
        Self {
            id: generate_job_id(),
            name: name.into(),
            payload: payload.into(),
            priority: JobPriority::Normal,
            max_retries: 3,
            retry_count: 0,
            status: JobStatus::Pending,
            created_at: now,
            scheduled_at: None,
            started_at: None,
            completed_at: None,
            error: None,
            retry_delay_ms: 1000,
        }
    }

    pub fn priority(mut self, p: JobPriority) -> Self {
        self.priority = p;
        self
    }

    pub fn max_retries(mut self, n: u32) -> Self {
        self.max_retries = n;
        self
    }

    pub fn retry_delay(mut self, ms: u64) -> Self {
        self.retry_delay_ms = ms;
        self
    }

    pub fn schedule_after(mut self, delay: Duration) -> Self {
        self.scheduled_at = Some(now_ms() + delay.as_millis() as u64);
        self.status = JobStatus::Scheduled;
        self
    }

    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }

    pub fn is_ready(&self) -> bool {
        match self.status {
            JobStatus::Pending => true,
            JobStatus::Scheduled => {
                if let Some(at) = self.scheduled_at {
                    now_ms() >= at
                } else {
                    true
                }
            }
            JobStatus::Retrying => true,
            _ => false,
        }
    }

    /// Mark job as started
    pub fn mark_running(&mut self) {
        self.status = JobStatus::Running;
        self.started_at = Some(now_ms());
    }

    /// Mark job as completed
    pub fn mark_completed(&mut self) {
        self.status = JobStatus::Completed;
        self.completed_at = Some(now_ms());
    }

    /// Mark job as failed
    pub fn mark_failed(&mut self, error: &str) {
        self.retry_count += 1;
        self.error = Some(error.to_string());
        if self.retry_count < self.max_retries {
            self.status = JobStatus::Retrying;
        } else {
            self.status = JobStatus::Failed;
            self.completed_at = Some(now_ms());
        }
    }

    /// Duration in ms (from start to completion)
    pub fn duration_ms(&self) -> Option<u64> {
        match (self.started_at, self.completed_at) {
            (Some(start), Some(end)) => Some(end.saturating_sub(start)),
            _ => None,
        }
    }

    pub fn to_json(&self) -> String {
        format!(
            "{{\"id\":\"{}\",\"name\":\"{}\",\"status\":\"{:?}\",\"retries\":{},\"priority\":{:?}}}",
            self.id, self.name, self.status, self.retry_count, self.priority as u8
        )
    }
}

// ─── Job Queue ──────────────────────────────────────────────

/// In-memory job queue with priority ordering
pub struct JobQueue {
    pending: Arc<Mutex<VecDeque<Job>>>,
    jobs: Arc<DashMap<String, Job>>,
    max_concurrency: usize,
    active_count: Arc<Mutex<usize>>,
    notify: Arc<Notify>,
}

impl JobQueue {
    pub fn new(max_concurrency: usize) -> Self {
        Self {
            pending: Arc::new(Mutex::new(VecDeque::new())),
            jobs: Arc::new(DashMap::new()),
            max_concurrency,
            active_count: Arc::new(Mutex::new(0)),
            notify: Arc::new(Notify::new()),
        }
    }

    /// Add a job to the queue
    pub async fn enqueue(&self, job: Job) -> String {
        let id = job.id.clone();
        self.jobs.insert(id.clone(), job.clone());
        let mut queue = self.pending.lock().await;

        // Insert based on priority (higher priority first)
        let pos = queue
            .iter()
            .position(|j| j.priority < job.priority)
            .unwrap_or(queue.len());
        queue.insert(pos, job);

        self.notify.notify_one();
        id
    }

    /// Get the next ready job from the queue
    pub async fn dequeue(&self) -> Option<Job> {
        let mut queue = self.pending.lock().await;
        let pos = queue.iter().position(|j| j.is_ready())?;
        let mut job = queue.remove(pos)?;
        job.mark_running();
        self.jobs.insert(job.id.clone(), job.clone());
        let mut count = self.active_count.lock().await;
        *count += 1;
        Some(job)
    }

    /// Mark a job as completed
    pub async fn complete(&self, job_id: &str) {
        if let Some(mut entry) = self.jobs.get_mut(job_id) {
            entry.mark_completed();
        }
        let mut count = self.active_count.lock().await;
        *count = count.saturating_sub(1);
    }

    /// Mark a job as failed (will re-enqueue if retries remain)
    pub async fn fail(&self, job_id: &str, error: &str) {
        let should_retry = if let Some(mut entry) = self.jobs.get_mut(job_id) {
            entry.mark_failed(error);
            entry.status == JobStatus::Retrying
        } else {
            false
        };

        let mut count = self.active_count.lock().await;
        *count = count.saturating_sub(1);

        if should_retry {
            if let Some(job) = self.jobs.get(job_id) {
                let mut queue = self.pending.lock().await;
                queue.push_back(job.clone());
            }
        }
    }

    /// Get job by ID
    pub fn get_job(&self, id: &str) -> Option<Job> {
        self.jobs.get(id).map(|j| j.clone())
    }

    /// Get queue statistics
    pub async fn stats(&self) -> QueueStats {
        let queue = self.pending.lock().await;
        let active = *self.active_count.lock().await;
        let mut completed = 0;
        let mut failed = 0;
        for entry in self.jobs.iter() {
            match entry.status {
                JobStatus::Completed => completed += 1,
                JobStatus::Failed => failed += 1,
                _ => {}
            }
        }
        QueueStats {
            pending: queue.len(),
            active,
            completed,
            failed,
            total: self.jobs.len(),
            max_concurrency: self.max_concurrency,
        }
    }

    /// Get the concurrency limit
    pub fn max_concurrency(&self) -> usize {
        self.max_concurrency
    }
}

/// Queue statistics
#[derive(Debug, Clone)]
pub struct QueueStats {
    pub pending: usize,
    pub active: usize,
    pub completed: usize,
    pub failed: usize,
    pub total: usize,
    pub max_concurrency: usize,
}

impl QueueStats {
    pub fn to_json(&self) -> String {
        format!(
            "{{\"pending\":{},\"active\":{},\"completed\":{},\"failed\":{},\"total\":{},\"max_concurrency\":{}}}",
            self.pending, self.active, self.completed, self.failed, self.total, self.max_concurrency
        )
    }
}

// ─── Cron-like Scheduler ────────────────────────────────────

/// A recurring job schedule
#[derive(Debug, Clone)]
pub struct RecurringJob {
    pub name: String,
    pub interval: Duration,
    pub payload: String,
    pub max_retries: u32,
}

impl RecurringJob {
    pub fn new(name: impl Into<String>, interval: Duration) -> Self {
        Self {
            name: name.into(),
            interval,
            payload: "{}".to_string(),
            max_retries: 3,
        }
    }

    pub fn payload(mut self, p: impl Into<String>) -> Self {
        self.payload = p.into();
        self
    }

    /// Create a Job instance from this recurring definition
    pub fn to_job(&self) -> Job {
        Job::new(&self.name, &self.payload).max_retries(self.max_retries)
    }
}

// ─── Helpers ────────────────────────────────────────────────

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn generate_job_id() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("job_{:x}_{:x}", now.as_secs(), now.subsec_nanos())
}

// ─── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_creation() {
        let job = Job::new("send_email", "{\"to\":\"user@example.com\"}");
        assert_eq!(job.name, "send_email");
        assert_eq!(job.status, JobStatus::Pending);
        assert!(job.id.starts_with("job_"));
    }

    #[test]
    fn test_job_priority() {
        let job = Job::new("task", "{}").priority(JobPriority::High);
        assert_eq!(job.priority, JobPriority::High);
        assert!(JobPriority::Critical > JobPriority::High);
        assert!(JobPriority::High > JobPriority::Normal);
    }

    #[test]
    fn test_job_lifecycle() {
        let mut job = Job::new("task", "{}");
        assert!(job.is_ready());
        job.mark_running();
        assert_eq!(job.status, JobStatus::Running);
        assert!(!job.is_ready());
        job.mark_completed();
        assert_eq!(job.status, JobStatus::Completed);
    }

    #[test]
    fn test_job_retry() {
        let mut job = Job::new("task", "{}").max_retries(2);
        job.mark_running();
        job.mark_failed("error 1");
        assert_eq!(job.status, JobStatus::Retrying);
        assert_eq!(job.retry_count, 1);
        assert!(job.can_retry());

        job.mark_running();
        job.mark_failed("error 2");
        assert_eq!(job.status, JobStatus::Failed);
        assert!(!job.can_retry());
    }

    #[test]
    fn test_job_no_retry() {
        let mut job = Job::new("task", "{}").max_retries(0);
        job.mark_running();
        job.mark_failed("fatal");
        assert_eq!(job.status, JobStatus::Failed);
    }

    #[test]
    fn test_job_schedule() {
        let job = Job::new("task", "{}").schedule_after(Duration::from_secs(60));
        assert_eq!(job.status, JobStatus::Scheduled);
        assert!(!job.is_ready()); // scheduled in the future
    }

    #[tokio::test]
    async fn test_queue_enqueue_dequeue() {
        let queue = JobQueue::new(2);
        let id = queue.enqueue(Job::new("test", "{}")).await;
        assert!(!id.is_empty());

        let job = queue.dequeue().await.unwrap();
        assert_eq!(job.name, "test");
        assert_eq!(job.status, JobStatus::Running);
    }

    #[tokio::test]
    async fn test_queue_priority_ordering() {
        let queue = JobQueue::new(2);
        queue.enqueue(Job::new("low", "{}").priority(JobPriority::Low)).await;
        queue.enqueue(Job::new("high", "{}").priority(JobPriority::High)).await;
        queue.enqueue(Job::new("normal", "{}").priority(JobPriority::Normal)).await;

        let first = queue.dequeue().await.unwrap();
        assert_eq!(first.name, "high");

        let second = queue.dequeue().await.unwrap();
        assert_eq!(second.name, "normal");
    }

    #[tokio::test]
    async fn test_queue_complete() {
        let queue = JobQueue::new(2);
        let id = queue.enqueue(Job::new("task", "{}")).await;
        queue.dequeue().await;
        queue.complete(&id).await;

        let job = queue.get_job(&id).unwrap();
        assert_eq!(job.status, JobStatus::Completed);
    }

    #[tokio::test]
    async fn test_queue_fail_retry() {
        let queue = JobQueue::new(2);
        let id = queue.enqueue(Job::new("task", "{}").max_retries(2)).await;
        queue.dequeue().await;
        queue.fail(&id, "oops").await;

        let job = queue.get_job(&id).unwrap();
        assert_eq!(job.status, JobStatus::Retrying);
    }

    #[tokio::test]
    async fn test_queue_stats() {
        let queue = JobQueue::new(4);
        queue.enqueue(Job::new("a", "{}")).await;
        queue.enqueue(Job::new("b", "{}")).await;
        let stats = queue.stats().await;
        assert_eq!(stats.pending, 2);
        assert_eq!(stats.max_concurrency, 4);
    }

    #[test]
    fn test_recurring_job() {
        let recurring = RecurringJob::new("cleanup", Duration::from_secs(3600))
            .payload("{\"table\":\"sessions\"}");
        let job = recurring.to_job();
        assert_eq!(job.name, "cleanup");
        assert!(job.payload.contains("sessions"));
    }

    #[test]
    fn test_job_to_json() {
        let job = Job::new("test", "{}");
        let json = job.to_json();
        assert!(json.contains("\"test\""));
        assert!(json.contains("Pending"));
    }

    #[test]
    fn test_stats_to_json() {
        let stats = QueueStats {
            pending: 5, active: 2, completed: 10, failed: 1, total: 18, max_concurrency: 4,
        };
        let json = stats.to_json();
        assert!(json.contains("\"pending\":5"));
        assert!(json.contains("\"completed\":10"));
    }
}
