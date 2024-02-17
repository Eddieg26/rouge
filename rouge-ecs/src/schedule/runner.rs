use super::graph;
use crate::{
    available_threads,
    tasks::{barrier::JobBarrier, ScopedTaskPool},
    world::World,
};
use std::sync::{Arc, Mutex};

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
        for row in graph.hierarchy() {
            let available_threads = available_threads();
            if available_threads > 1 {
                let local_nodes = row
                    .iter()
                    .filter(|id| graph.nodes()[***id].is_local())
                    .collect::<Vec<_>>();
                let row = row
                    .iter()
                    .filter(|id| !local_nodes.contains(id))
                    .collect::<Vec<_>>();

                let num_threads = row.len().min(available_threads);
                ScopedTaskPool::new(num_threads, |sender| {
                    let (barrier, lock) = JobBarrier::new(row.len());
                    let barrier = Arc::new(Mutex::new(barrier));

                    for node in &row {
                        let barrier = barrier.clone();
                        let node = &graph.nodes()[node.id()];

                        sender.send(move || {
                            node.run(world);

                            let mut barrier_lock = barrier.lock().expect("Failed to lock barrier");
                            barrier_lock.notify();
                        });
                    }

                    sender.join();

                    for node in &local_nodes {
                        let node = &graph.nodes()[node.id()];

                        node.run(world);
                    }

                    lock.wait(barrier.lock().expect("Failed to lock barrier"));
                });
            } else {
                for node in row {
                    let node = &graph.nodes()[node.id()];

                    node.run(world);
                }
            }
        }
    }
}
