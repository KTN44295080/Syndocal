//! Fixed-clock orchestration for independent resource-neutral chains.
use super::*;

/// Inline storage for the transaction's bounded zero-to-three jobs. Arrays give
/// the canonical encoder a contiguous slice without heap bookkeeping.
#[derive(Debug)]
pub(crate) enum StackBatch<T> {
    Empty,
    One([T; 1]),
    Two([T; 2]),
    Three([T; 3]),
}

impl<T> StackBatch<T> {
    pub(crate) fn from_fn(len: usize, f: impl FnMut(usize) -> T) -> Self {
        match len {
            0 => Self::Empty,
            1 => Self::One(std::array::from_fn(f)),
            2 => Self::Two(std::array::from_fn(f)),
            3 => Self::Three(std::array::from_fn(f)),
            _ => unreachable!("ISF transaction must contain zero to three jobs"),
        }
    }

    pub(crate) fn push(&mut self, item: T) {
        assert!(self.len() < 3, "ISF batch exceeds three jobs");
        *self = match std::mem::replace(self, Self::Empty) {
            Self::Empty => Self::One([item]),
            Self::One([a]) => Self::Two([a, item]),
            Self::Two([a, b]) => Self::Three([a, b, item]),
            Self::Three(_) => unreachable!("ISF batch exceeds three jobs"),
        };
    }

    pub(crate) fn drain(&mut self) -> <Self as IntoIterator>::IntoIter {
        std::mem::replace(self, Self::Empty).into_iter()
    }

    pub(crate) fn map<U>(self, f: impl FnMut(T) -> U) -> StackBatch<U> {
        match self {
            Self::Empty => StackBatch::Empty,
            Self::One(items) => StackBatch::One(items.map(f)),
            Self::Two(items) => StackBatch::Two(items.map(f)),
            Self::Three(items) => StackBatch::Three(items.map(f)),
        }
    }
}

impl<T> std::ops::Deref for StackBatch<T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        match self {
            Self::Empty => &[],
            Self::One(items) => items,
            Self::Two(items) => items,
            Self::Three(items) => items,
        }
    }
}

impl<T> std::ops::DerefMut for StackBatch<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        match self {
            Self::Empty => &mut [],
            Self::One(items) => items,
            Self::Two(items) => items,
            Self::Three(items) => items,
        }
    }
}

impl<T> IntoIterator for StackBatch<T> {
    type Item = T;
    type IntoIter = std::iter::Flatten<std::array::IntoIter<Option<T>, 3>>;
    fn into_iter(self) -> Self::IntoIter {
        match self {
            Self::Empty => [None, None, None],
            Self::One([a]) => [Some(a), None, None],
            Self::Two([a, b]) => [Some(a), Some(b), None],
            Self::Three([a, b, c]) => [Some(a), Some(b), Some(c)],
        }
        .into_iter()
        .flatten()
    }
}

#[cfg(test)]
impl<T: PartialEq> PartialEq<Vec<T>> for StackBatch<T> {
    fn eq(&self, other: &Vec<T>) -> bool {
        self[..] == other[..]
    }
}

pub(crate) struct IndependentChain<'a> {
    pub frame: Arc<VideoFrame>,
    pub stages: &'a [IsfStageInputs<'a>],
    pub keys: &'a [IsfInstanceKey],
    pub time_seconds: f32,
    pub time_delta_seconds: f32,
    pub date: [f32; 4],
}

impl IsfGpuRuntime {
    /// None means no job ran and the caller still owns sequential execution.
    /// A failed admitted batch is fully recovered here at its reserved clocks.
    pub(crate) fn apply_independent_chains(
        &mut self,
        chains: &[IndependentChain<'_>],
    ) -> Option<StackBatch<(VideoFrame, Vec<(usize, IsfRuntimeError)>)>> {
        if !(2..=3).contains(&chains.len()) {
            return None;
        }
        #[cfg(test)]
        if let Err(error) = self.reject_ownerless_native_entry() {
            return Some(StackBatch::from_fn(chains.len(), |index| {
                let chain = &chains[index];
                (
                    chain.frame.as_ref().clone(),
                    (0..chain.stages.len())
                        .map(|stage| (stage, error.clone()))
                        .collect(),
                )
            }));
        }
        if !(2..=3).contains(&chains.len())
            || chains.iter().any(|chain| {
                chain.stages.is_empty()
                    || chain.keys.len() != chain.stages.len()
                    || !date::valid_isf_date(chain.date)
                    || !chain
                        .stages
                        .iter()
                        .all(IsfStageInputs::can_share_host_buffers)
            })
            || chains.iter().enumerate().any(|(index, chain)| {
                chains[..index]
                    .iter()
                    .any(|earlier| earlier.frame.layer_id == chain.frame.layer_id)
            })
        {
            return None;
        }
        let clocks = self.reserve_independent_clocks(chains.len());
        let requests = StackBatch::from_fn(chains.len(), |index| {
            let chain = &chains[index];
            pass_runtime::IndependentStack {
                frame: Arc::clone(&chain.frame),
                stages: chain.stages,
                keys: Cow::Borrowed(chain.keys),
                frame_index: clocks.at(index),
                time_seconds: chain.time_seconds,
                time_delta_seconds: chain.time_delta_seconds,
                date: chain.date,
            }
        });
        let attempted = self.try_render_independent_stacks(&requests)?;
        let results = match attempted {
            Ok(frames) => frames.map(|frame| (frame, Vec::new())),
            Err(_) => StackBatch::from_fn(chains.len(), |index| {
                let chain = &chains[index];
                // Only resource-neutral work can reach this retry. The pass
                // owner has drained/unmapped it, or is poisoned and refuses
                // GPU work. Never retry a target-backed feedback stage.
                self.apply_stack_stage_local_at_frame_index(
                    &chain.frame,
                    chain.stages,
                    chain.time_seconds,
                    chain.time_delta_seconds,
                    chain.date,
                    chain.keys,
                    clocks.at(index),
                )
            }),
        };
        self.commit_independent_clocks(clocks);
        Some(results)
    }
}

/// One logical clock owner for the CPU and native independent executors.
/// Taking this value is observational; only admitted work commits it.
pub(crate) struct IndependentClockReservation {
    first: u32,
    count: usize,
    #[cfg(test)]
    epoch: Arc<()>,
}

impl IndependentClockReservation {
    pub(crate) fn at(&self, index: usize) -> u32 {
        assert!(index < self.count);
        self.first.wrapping_add(index as u32)
    }
}

impl IsfGpuRuntime {
    pub(crate) fn reserve_independent_clocks(&self, count: usize) -> IndependentClockReservation {
        assert!((1..=3).contains(&count));
        IndependentClockReservation {
            first: self.frame_index,
            count,
            #[cfg(test)]
            epoch: Arc::clone(&self.resources.native_clock_epoch),
        }
    }

    #[cfg(test)]
    pub(crate) fn native_clocks_current(&self, clocks: &IndependentClockReservation) -> bool {
        self.frame_index == clocks.first
            && Arc::ptr_eq(&self.resources.native_clock_epoch, &clocks.epoch)
    }

    pub(crate) fn commit_independent_clocks(&mut self, clocks: IndependentClockReservation) {
        assert_eq!(
            self.frame_index, clocks.first,
            "a logical chain cannot be committed twice"
        );
        self.advance_frame_index(clocks.count as u32);
    }
}

#[cfg(test)]
#[path = "isf_independent_runtime_tests.rs"]
mod tests;
