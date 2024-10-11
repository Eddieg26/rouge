use crate::{core::resource::Resource, system::SystemArg};
use std::{collections::VecDeque, future::Future, pin::Pin, sync::Arc};

pub type ScopedTask<'a> = Box<dyn FnOnce() + Send + 'a>;

pub fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

pub struct ScopedTaskPool<'a> {
    size: usize,
    queue: VecDeque<ScopedTask<'a>>,
}

impl<'a> ScopedTaskPool<'a> {
    pub fn new(size: usize) -> Self {
        ScopedTaskPool {
            size,
            queue: VecDeque::new(),
        }
    }

    pub fn spawn(&mut self, task: impl FnOnce() + Send + 'a) -> &mut Self {
        self.queue.push_back(Box::new(task));
        if self.queue.len() >= self.size {
            self.run();
        }
        self
    }

    pub fn run(&mut self) {
        while !self.queue.is_empty() {
            let len = self.queue.len().min(self.size);
            let tasks = self.queue.drain(..len).collect::<Vec<_>>();
            std::thread::scope(move |scope| {
                for task in tasks {
                    scope.spawn(move || task());
                }
            });
        }
    }
}

#[derive(Clone)]
pub struct TaskPool {
    pool: Arc<threadpool::ThreadPool>,
}

impl TaskPool {
    pub fn new(size: usize) -> Self {
        TaskPool {
            pool: Arc::new(threadpool::Builder::new().num_threads(size).build()),
        }
    }

    pub fn size(&self) -> usize {
        self.pool.max_count()
    }

    pub fn spawn(&self, task: impl FnOnce() + Send + 'static) {
        self.pool.execute(move || task());
    }
}

impl Resource for TaskPool {}

impl Default for TaskPool {
    fn default() -> Self {
        let pool = TaskPool::new(num_cpus());
        pool
    }
}

pub struct AsyncTaskPool<'a, Output: 'static> {
    pool: Vec<Pin<Box<dyn futures::Future<Output = Output> + 'a>>>,
}

impl<'a, Output: 'static> AsyncTaskPool<'a, Output> {
    pub fn new() -> Self {
        AsyncTaskPool { pool: Vec::new() }
    }

    pub fn spawn(&mut self, future: impl futures::Future<Output = Output> + 'a) {
        self.pool.push(Box::pin(future));
    }

    pub fn run(&mut self) -> impl Future<Output = Vec<Output>> + 'a {
        let futures = self
            .pool
            .drain(..)
            .map(|a| async { a.await })
            .collect::<Vec<_>>();

        futures::future::join_all(futures)
    }
}

impl SystemArg for &TaskPool {
    type Item<'a> = &'a TaskPool;

    fn get<'a>(world: &'a crate::world::cell::WorldCell) -> Self::Item<'a> {
        world.get().tasks()
    }
}
