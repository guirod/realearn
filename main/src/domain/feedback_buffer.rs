use crate::domain::{MappingCompartment, MappingId};

use std::collections::HashSet;
use std::time::{Duration, Instant};

const BUFFER_DURATION: Duration = Duration::from_millis(10);

#[derive(Debug)]
pub struct FeedbackBuffer {
    last_buffer_start: Instant,
    buffered_mapping_ids: HashSet<(MappingCompartment, MappingId)>,
}

impl Default for FeedbackBuffer {
    fn default() -> Self {
        Self {
            last_buffer_start: Instant::now(),
            buffered_mapping_ids: Default::default(),
        }
    }
}

impl FeedbackBuffer {
    pub fn reset_all(&mut self) {
        self.buffered_mapping_ids.clear();
        self.last_buffer_start = Instant::now();
    }

    pub fn reset_all_in_compartment(&mut self, compartment: MappingCompartment) {
        self.buffered_mapping_ids
            .retain(|(c, _id)| *c != compartment);
        self.last_buffer_start = Instant::now();
    }

    pub fn len(&self) -> usize {
        self.buffered_mapping_ids.len()
    }

    pub fn buffer_feedback_for_mapping(
        &mut self,
        compartment: MappingCompartment,
        mapping_id: MappingId,
    ) {
        self.buffered_mapping_ids.insert((compartment, mapping_id));
    }

    pub fn poll(&mut self) -> Option<HashSet<(MappingCompartment, MappingId)>> {
        if Instant::now() - self.last_buffer_start <= BUFFER_DURATION {
            return None;
        }
        self.last_buffer_start = Instant::now();
        let old_ids = std::mem::replace(&mut self.buffered_mapping_ids, HashSet::new());
        Some(old_ids)
    }
}
