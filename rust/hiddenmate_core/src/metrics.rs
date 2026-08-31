use std::time::Duration;

/// 初形候補世界の明示列挙に関する計測値。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EnumerationMetrics {
    pub world_count: usize,
    pub elapsed: Duration,
}

/// 候補世界を使った手順探索に関する計測値。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SolveMetrics {
    pub initial_world_count: usize,
    pub peak_world_count: usize,
    pub visited_state_count: usize,
    pub generated_transition_count: usize,
    pub generated_successor_world_count: usize,
    pub move_generation_elapsed: Duration,
    pub transition_elapsed: Duration,
    pub total_elapsed: Duration,
}

impl SolveMetrics {
    pub(crate) fn new(initial_world_count: usize) -> Self {
        Self {
            initial_world_count,
            peak_world_count: initial_world_count,
            ..Self::default()
        }
    }

    pub(crate) fn visit_state(&mut self, world_count: usize) {
        self.visited_state_count += 1;
        self.peak_world_count = self.peak_world_count.max(world_count);
    }

    pub(crate) fn record_successor(&mut self, world_count: usize) {
        self.generated_transition_count += 1;
        self.generated_successor_world_count += world_count;
        self.peak_world_count = self.peak_world_count.max(world_count);
    }
}
