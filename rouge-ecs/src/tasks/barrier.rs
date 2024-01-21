use std::sync::{Arc, Condvar, Mutex, MutexGuard};

pub struct JobBarrier {
    count: usize,
    total: usize,
    condvar: Arc<Condvar>,
}

impl JobBarrier {
    pub fn new<'a>(total: usize) -> (Self, BarrierLock) {
        let condvar = Arc::new(Condvar::new());
        let barrier = Self {
            count: 0,
            total,
            condvar: condvar.clone(),
        };

        let lock = BarrierLock::new(condvar);

        (barrier, lock)
    }

    pub fn notify(&mut self) {
        self.count += 1;

        if self.count >= self.total {
            self.condvar.notify_all();
        }
    }
}

pub struct BarrierLock {
    condvar: Arc<Condvar>,
    guard: Arc<Mutex<()>>,
}

impl BarrierLock {
    fn new(condvar: Arc<Condvar>) -> Self {
        let guard = Arc::new(Mutex::new(()));
        Self { condvar, guard }
    }

    pub fn wait(&self, barrier: MutexGuard<JobBarrier>) {
        let count = barrier.count;
        let total = barrier.total;

        if count < total {
            std::mem::drop(barrier);
            let guard = self.guard.lock().unwrap();
            let _ = self.condvar.wait(guard);
        }
    }
}
