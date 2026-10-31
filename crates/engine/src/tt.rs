use shaxmat_core::Move;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bound {
    Exact,
    Lower,
    Upper,
}

#[derive(Debug, Clone, Copy)]
pub struct Entry {
    key: u64,
    pub mv: Option<Move>,
    pub score: i32,
    pub depth: i32,
    pub bound: Bound,
}

/// A fixed-size, always-allocated hash table of search results.
pub struct TranspositionTable {
    entries: Vec<Option<Entry>>,
    mask: usize,
}

impl TranspositionTable {
    /// Creates a table with `2^bits` entries.
    pub fn new(bits: u32) -> Self {
        TranspositionTable {
            entries: vec![None; 1 << bits],
            mask: (1 << bits) - 1,
        }
    }

    pub fn clear(&mut self) {
        self.entries.fill(None);
    }

    pub fn probe(&self, key: u64) -> Option<Entry> {
        self.entries[key as usize & self.mask].filter(|entry| entry.key == key)
    }

    /// Keeps the deeper entry for the same position; another position replaces.
    pub fn store(&mut self, key: u64, mv: Option<Move>, score: i32, depth: i32, bound: Bound) {
        let slot = &mut self.entries[key as usize & self.mask];
        if let Some(old) = slot
            && old.key == key
            && old.depth > depth
        {
            return;
        }
        *slot = Some(Entry {
            key,
            mv,
            score,
            depth,
            bound,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_deeper_results_and_detects_foreign_keys() {
        let mut table = TranspositionTable::new(4);
        table.store(5, None, 10, 6, Bound::Exact);
        table.store(5, None, 20, 3, Bound::Lower);
        assert_eq!(table.probe(5).unwrap().score, 10);
        // The same slot, another position.
        assert!(table.probe(5 + 16).is_none());
        table.store(5 + 16, None, 30, 1, Bound::Upper);
        assert!(table.probe(5).is_none());
        table.clear();
        assert!(table.probe(5 + 16).is_none());
    }
}
