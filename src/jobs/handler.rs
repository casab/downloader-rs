//! Job handler traits and registry.

use crate::jobs::{Job, JobResult, JobType};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

/// Trait for job handlers.
#[async_trait]
pub trait JobHandler: Send + Sync {
    /// Execute the job and return a result.
    async fn execute(&self, job: &Job) -> anyhow::Result<JobResult>;

    /// Get the job type this handler processes.
    fn job_type(&self) -> JobType;
}

/// Registry of job handlers.
pub struct JobHandlers {
    handlers: HashMap<JobType, Arc<dyn JobHandler>>,
}

impl JobHandlers {
    /// Create a new empty handler registry.
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Register a handler for a job type.
    pub fn register<H: JobHandler + 'static>(&mut self, handler: H) {
        let job_type = handler.job_type();
        self.handlers.insert(job_type, Arc::new(handler));
    }

    /// Get the handler for a job type.
    pub fn get(&self, job_type: &JobType) -> Option<Arc<dyn JobHandler>> {
        self.handlers.get(job_type).cloned()
    }

    /// Check if a handler is registered for a job type.
    pub fn has_handler(&self, job_type: &JobType) -> bool {
        self.handlers.contains_key(job_type)
    }

    /// Get all registered job types.
    pub fn registered_types(&self) -> Vec<JobType> {
        self.handlers.keys().copied().collect()
    }
}

impl Default for JobHandlers {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockHandler {
        job_type: JobType,
    }

    #[async_trait]
    impl JobHandler for MockHandler {
        async fn execute(&self, _job: &Job) -> anyhow::Result<JobResult> {
            Ok(JobResult::empty())
        }

        fn job_type(&self) -> JobType {
            self.job_type
        }
    }

    #[test]
    fn test_handler_registry() {
        let mut handlers = JobHandlers::new();

        handlers.register(MockHandler {
            job_type: JobType::Download,
        });
        handlers.register(MockHandler {
            job_type: JobType::Email,
        });

        assert!(handlers.has_handler(&JobType::Download));
        assert!(handlers.has_handler(&JobType::Email));
        assert!(!handlers.has_handler(&JobType::Cleanup));

        let registered = handlers.registered_types();
        assert_eq!(registered.len(), 2);
    }
}
