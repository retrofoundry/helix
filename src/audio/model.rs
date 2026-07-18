//! Descriptor tracker for the audio interface.
//!
//! A non-gating tracker of in-flight guest audio DMA buffers. Each entry records
//! the guest source-frame span a DMA covers, so the current DMA's remaining
//! byte length can be computed from Arie's monotonic `retired_source_position`.
//! The tracker never rejects a push and never gates a submit: Arie's own software
//! queue is the backpressure. It exists so `osAiGetLength` can be reported
//! accurately. It carries no Arie API dependency; its API is crate-private and
//! owned only by the `AudioRuntime`.

use std::collections::VecDeque;

/// Bytes emitted per accepted stereo source frame (two interleaved `i16`).
const BYTES_PER_FRAME: u32 = 4;

/// A non-gating FIFO of in-flight guest audio DMA spans.
///
/// Each entry is the half-open source-frame span `(start, end)` a single DMA
/// covers; the front entry is the current DMA. Pushes never reject — there is
/// no capacity — and entries are reaped from the front once Arie's monotonic
/// retired position has passed their `end`. Only the current DMA's remaining
/// length is ever reported, so the tracker never surfaces total occupancy.
#[derive(Clone, Debug, Default)]
pub(crate) struct DescriptorTracker {
    spans: VecDeque<(u64, u64)>,
}

impl DescriptorTracker {
    /// Create an empty tracker with no in-flight DMAs.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Append an in-flight DMA span `(start, end)`.
    ///
    /// Never rejects — there is no capacity, so a submit can always push straight
    /// into Arie. A non-positive span carries no frames and would only corrupt
    /// `current_dma_remaining_bytes`, so it is dropped; this guard is
    /// well-formedness only and never gates a real DMA.
    pub(crate) fn push(&mut self, start: u64, end: u64) {
        if end > start {
            self.spans.push_back((start, end));
        }
    }

    /// Pop every front span fully retired at `retired` (`end <= retired`).
    ///
    /// Retirement is monotonic, so removal only ever happens from the front.
    pub(crate) fn reap(&mut self, retired: u64) {
        while let Some(&(_, end)) = self.spans.front() {
            if end <= retired {
                self.spans.pop_front();
            } else {
                break;
            }
        }
    }

    /// Reap at `retired`, then return the bytes remaining in the current DMA.
    ///
    /// `retired` is clamped into the front DMA's `[start, end]` span before
    /// subtracting, so the result is at most that one DMA's length
    /// (`(end - start) * 4`) even if `retired` predates this DMA's start (the
    /// tracker does not assume the retired position is contiguous). Reap
    /// guarantees `end > retired` for a present front, so the clamp yields
    /// `retired` there; a `retired` below `start` clamps up to `start` (full
    /// DMA); `0` when empty. All arithmetic is `u64` with a saturating final
    /// conversion.
    pub(crate) fn current_dma_remaining_bytes(&mut self, retired: u64) -> u32 {
        self.reap(retired);
        match self.spans.front() {
            Some(&(start, end)) => {
                let remaining_frames = end.saturating_sub(retired.clamp(start, end));
                let remaining_bytes = remaining_frames.saturating_mul(u64::from(BYTES_PER_FRAME));
                u32::try_from(remaining_bytes).unwrap_or(u32::MAX)
            }
            None => 0,
        }
    }

    /// The number of in-flight DMAs currently tracked. Test-only: lets a test
    /// check that reaping keeps the tracker from growing unbounded.
    #[cfg(test)]
    pub(crate) fn in_flight_dmas(&self) -> usize {
        self.spans.len()
    }

    /// Empty the tracker, dropping all in-flight DMA spans.
    pub(crate) fn clear(&mut self) {
        self.spans.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_dma_is_bounded_to_one_dma_even_when_retired_predates_start() {
        // A DMA spanning [100, 628) is 528 frames = 2112 bytes. A retired position
        // before its start must clamp to `start` and report exactly that one DMA's
        // length, not `(end - retired) * 4 = 2512`.
        let mut t = DescriptorTracker::new();
        t.push(100, 628);
        assert_eq!(t.current_dma_remaining_bytes(0), 2112);
        assert_eq!(t.current_dma_remaining_bytes(100), 2112);
        assert_eq!(t.current_dma_remaining_bytes(300), (628 - 300) * 4);
        assert_eq!(t.current_dma_remaining_bytes(628), 0); // fully retired → reaped → empty
    }

    #[test]
    fn only_the_current_front_dma_is_reported_never_the_sum() {
        let mut t = DescriptorTracker::new();
        t.push(0, 528);
        t.push(528, 1056);
        assert_eq!(t.current_dma_remaining_bytes(0), 528 * 4); // front only, not 1056*4
        assert_eq!(t.in_flight_dmas(), 2);
        assert_eq!(t.current_dma_remaining_bytes(528), 528 * 4); // DMA #2 becomes front
        assert_eq!(t.in_flight_dmas(), 1);
    }

    #[test]
    fn non_positive_span_is_dropped_and_never_gates() {
        let mut t = DescriptorTracker::new();
        t.push(500, 500); // zero span
        t.push(600, 500); // reversed
        assert_eq!(t.in_flight_dmas(), 0);
        assert_eq!(t.current_dma_remaining_bytes(0), 0);
    }
}
