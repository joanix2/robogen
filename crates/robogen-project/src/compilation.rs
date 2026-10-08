use crate::{MassDiagnostic, Project};
use robogen_domain::{Diagnostic, PartId};
use robogen_topopt::execution::ExecutionSettings;
use std::sync::{
    atomic::{AtomicU64, AtomicUsize, Ordering},
    mpsc::{self, Receiver, SyncSender},
    Arc, Mutex,
};

type BuildResult = Result<Project, Vec<Diagnostic>>;

struct Job {
    revision: u64,
    operation: Operation,
}

enum Operation {
    Prepare(String),
    Optimize(Box<Project>, PartId, ExecutionSettings),
}

pub(super) struct CompilationWorker {
    pending: Arc<Mutex<Option<Job>>>,
    revision: Arc<AtomicU64>,
    completed: Arc<AtomicUsize>,
    total: Arc<AtomicUsize>,
    wake: SyncSender<()>,
    results: Receiver<(u64, BuildResult)>,
    diagnostic: Arc<Mutex<Option<(u64, PartId, MassDiagnostic)>>>,
}

impl CompilationWorker {
    pub(super) fn new() -> std::io::Result<Self> {
        let pending = Arc::new(Mutex::new(None::<Job>));
        let revision = Arc::new(AtomicU64::new(0));
        let completed = Arc::new(AtomicUsize::new(0));
        let total = Arc::new(AtomicUsize::new(0));
        let (wake, wake_receiver) = mpsc::sync_channel(1);
        let (result_sender, results) = mpsc::channel();
        let diagnostic = Arc::new(Mutex::new(None));
        let worker_diagnostic = Arc::clone(&diagnostic);
        let worker_pending = Arc::clone(&pending);
        let worker_revision = Arc::clone(&revision);
        let worker_completed = Arc::clone(&completed);
        let worker_total = Arc::clone(&total);
        std::thread::Builder::new()
            .name("robogen-cad".into())
            .spawn(move || {
                while wake_receiver.recv().is_ok() {
                    let job = worker_pending
                        .lock()
                        .unwrap_or_else(|error| error.into_inner())
                        .take();
                    let Some(job) = job else {
                        continue;
                    };
                    let cancelled = || worker_revision.load(Ordering::Acquire) != job.revision;
                    if cancelled() {
                        continue;
                    }
                    let progress = |done, count| {
                        if !cancelled() {
                            worker_total.store(count, Ordering::Release);
                            worker_completed.store(done, Ordering::Release);
                        }
                    };
                    let result = match job.operation {
                        Operation::Prepare(source) => Project::from_source_controlled(
                            source,
                            job.revision,
                            &cancelled,
                            &progress,
                        ),
                        Operation::Optimize(mut project, part, settings) => project
                            .optimize_part_with_diagnostics(
                                part,
                                &settings,
                                job.revision,
                                &cancelled,
                                &progress,
                                &|event| {
                                    if !cancelled() {
                                        *worker_diagnostic
                                            .lock()
                                            .unwrap_or_else(|error| error.into_inner()) =
                                            Some((job.revision, part, event.clone()));
                                    }
                                },
                            )
                            .map(|()| *project),
                    };
                    if !cancelled() && result_sender.send((job.revision, result)).is_err() {
                        break;
                    }
                }
            })?;
        Ok(Self {
            pending,
            revision,
            completed,
            total,
            wake,
            results,
            diagnostic,
        })
    }

    pub(super) fn submit(&self, source: String) -> bool {
        self.enqueue(Operation::Prepare(source))
    }

    pub(super) fn optimize(
        &self,
        project: Project,
        part: PartId,
        settings: ExecutionSettings,
    ) -> bool {
        self.enqueue(Operation::Optimize(Box::new(project), part, settings))
    }

    fn enqueue(&self, operation: Operation) -> bool {
        let revision = self.revision.fetch_add(1, Ordering::AcqRel) + 1;
        self.diagnostic
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take();
        self.completed.store(0, Ordering::Release);
        self.total.store(0, Ordering::Release);
        *self
            .pending
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some(Job {
            revision,
            operation,
        });
        !matches!(
            self.wake.try_send(()),
            Err(mpsc::TrySendError::Disconnected(()))
        )
    }

    pub(super) fn poll(&self) -> Option<BuildResult> {
        let mut current = None;
        while let Ok((revision, result)) = self.results.try_recv() {
            if revision == self.revision.load(Ordering::Acquire) {
                current = Some(result);
            }
        }
        current
    }

    pub(super) fn progress(&self) -> (usize, usize) {
        (
            self.completed.load(Ordering::Acquire),
            self.total.load(Ordering::Acquire),
        )
    }

    pub(super) fn poll_diagnostic(&self) -> Option<(PartId, MassDiagnostic)> {
        let (revision, part, diagnostic) = self
            .diagnostic
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take()?;
        (revision == self.revision.load(Ordering::Acquire)).then_some((part, diagnostic))
    }

    pub(super) fn cancel(&self) {
        self.revision.fetch_add(1, Ordering::AcqRel);
        self.pending
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take();
    }
}

impl Drop for CompilationWorker {
    fn drop(&mut self) {
        self.cancel();
    }
}
