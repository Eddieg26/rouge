use super::graph;
use crate::{
    tasks::{barrier::JobBarrier, ScopedTaskPool},
    world::World,
};
use std::{
    num::NonZeroUsize,
    sync::{Arc, Mutex},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMode {
    Sequential,
    Parallel,
}

pub trait ScheduleRunner: Send + Sync {
    fn run(&self, graph: &graph::SystemGraph, world: &World);
}

pub struct SequentialRunner;

impl ScheduleRunner for SequentialRunner {
    fn run(&self, graph: &graph::SystemGraph, world: &World) {
        for row in graph.hierarchy() {
            for id in row {
                let node = &graph.nodes()[**id];

                node.run(world);
            }
        }
    }
}

pub struct ParallelRunner;

impl ScheduleRunner for ParallelRunner {
    fn run(&self, graph: &graph::SystemGraph, world: &World) {
        let available_threads = std::thread::available_parallelism()
            .unwrap_or(NonZeroUsize::new(1).unwrap())
            .into();
        for row in graph.hierarchy() {
            let num_threads = row.len().min(available_threads);

            ScopedTaskPool::new(num_threads, |sender| {
                let (barrier, lock) = JobBarrier::new(row.len());
                let barrier = Arc::new(Mutex::new(barrier));

                for node in row {
                    let barrier = barrier.clone();
                    let node = &graph.nodes()[node.id()];

                    sender.send(move || {
                        node.run(world);

                        barrier.lock().unwrap().notify();
                    });
                }

                sender.join();

                lock.wait(barrier.lock().unwrap());
            });
        }
    }
}
