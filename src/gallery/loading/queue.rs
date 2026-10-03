use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

pub(crate) const THUMB_WORKERS: usize = 4;

#[derive(Clone)]
pub(crate) struct ThumbRequest {
    pub(crate) id: u64,
    pub(crate) path: PathBuf,
    generation: u64,
    wanted: Arc<AtomicBool>,
}

impl ThumbRequest {
    pub(crate) fn is_wanted(&self) -> bool {
        self.wanted.load(Ordering::Acquire)
    }
}

#[derive(Default)]
pub(crate) struct ThumbRequests {
    generation: u64,
    next_id: u64,
    demand: HashSet<PathBuf>,
    pending: VecDeque<PathBuf>,
    active: HashMap<u64, ThumbRequest>,
}

impl ThumbRequests {
    /// Replace queued work with current viewport demand. Cancelled jobs retain
    /// their worker slot until completion, including across folder changes.
    pub(crate) fn set_demand(&mut self, generation: u64, paths: Vec<PathBuf>) {
        self.generation = generation;
        self.demand = paths.iter().cloned().collect();
        for request in self.active.values() {
            if request.generation != generation || !self.demand.contains(&request.path) {
                request.wanted.store(false, Ordering::Release);
            }
        }
        let mut queued = HashSet::new();
        self.pending = paths
            .into_iter()
            .filter(|path| {
                queued.insert(path.clone())
                    && !self.active.values().any(|request| {
                        request.generation == generation
                            && request.path == *path
                            && request.is_wanted()
                    })
            })
            .collect();
    }

    pub(crate) fn next_request(&mut self) -> Option<ThumbRequest> {
        if self.active.len() >= THUMB_WORKERS {
            return None;
        }
        let path = self.pending.pop_front()?;
        self.next_id += 1;
        let request = ThumbRequest {
            id: self.next_id,
            path,
            generation: self.generation,
            wanted: Arc::new(AtomicBool::new(true)),
        };
        self.active.insert(request.id, request.clone());
        Some(request)
    }

    /// Release a worker slot and return only a still-current result's path.
    pub(crate) fn complete(&mut self, id: u64) -> Option<PathBuf> {
        let request = self.active.remove(&id)?;
        (request.generation == self.generation
            && request.is_wanted()
            && self.demand.contains(&request.path))
        .then_some(request.path)
    }

    pub(crate) fn cancel(&mut self, generation: u64) {
        self.set_demand(generation, Vec::new());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(prefix: &str, count: usize) -> Vec<PathBuf> {
        (0..count)
            .map(|i| PathBuf::from(format!("{prefix}/{i}.jpg")))
            .collect()
    }

    #[test]
    fn bounds_workers_and_publishes_each_completion() {
        let mut queue = ThumbRequests::default();
        queue.set_demand(1, paths("a", 20));
        let jobs: Vec<_> = (0..THUMB_WORKERS)
            .map(|_| queue.next_request().unwrap())
            .collect();
        assert!(queue.next_request().is_none());
        assert_eq!(queue.complete(jobs[0].id), Some(jobs[0].path.clone()));
        assert_eq!(queue.next_request().unwrap().path, PathBuf::from("a/4.jpg"));
        assert!(queue.next_request().is_none());
    }

    #[test]
    fn replacing_viewport_cancels_obsolete_work_before_decode() {
        let mut queue = ThumbRequests::default();
        queue.set_demand(1, paths("a", 20));
        let job = queue.next_request().unwrap();
        queue.set_demand(1, paths("b", 10));
        assert!(!job.is_wanted());
        assert!(queue.complete(job.id).is_none());
        assert_eq!(queue.next_request().unwrap().path, PathBuf::from("b/0.jpg"));
    }

    #[test]
    fn folder_switch_keeps_the_global_worker_limit_and_rejects_old_results() {
        let mut queue = ThumbRequests::default();
        queue.set_demand(1, paths("same", 10));
        let jobs: Vec<_> = (0..THUMB_WORKERS)
            .map(|_| queue.next_request().unwrap())
            .collect();
        queue.cancel(2);
        queue.set_demand(2, paths("same", 10));
        assert!(queue.next_request().is_none());
        for job in jobs {
            assert!(!job.is_wanted());
            assert!(queue.complete(job.id).is_none());
        }
        assert_eq!(
            queue.next_request().unwrap().path,
            PathBuf::from("same/0.jpg")
        );
    }

    #[test]
    fn repeated_demand_does_not_duplicate_running_paths() {
        let mut queue = ThumbRequests::default();
        let path = PathBuf::from("a.jpg");
        queue.set_demand(1, vec![path.clone(), path.clone()]);
        let job = queue.next_request().unwrap();
        queue.set_demand(1, vec![path.clone()]);
        assert!(queue.next_request().is_none());
        assert!(job.is_wanted());
        assert_eq!(queue.complete(job.id), Some(path));
    }
}
