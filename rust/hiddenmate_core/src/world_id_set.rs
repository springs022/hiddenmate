/// 初形候補世界の安定ID集合を保持する可変長ビット集合。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct WorldIdSet {
    words: Vec<u64>,
}

impl WorldIdSet {
    pub(crate) fn with_capacity(world_count: usize) -> Self {
        Self {
            words: vec![0; world_count.div_ceil(64)],
        }
    }

    pub(crate) fn full(world_count: usize) -> Self {
        let mut result = Self::with_capacity(world_count);
        for id in 0..world_count {
            result.insert(id);
        }
        result
    }

    pub(crate) fn insert(&mut self, id: usize) {
        let word = id / 64;
        if word >= self.words.len() {
            self.words.resize(word + 1, 0);
        }
        self.words[word] |= 1u64 << (id % 64);
    }

    pub(crate) fn contains(&self, id: usize) -> bool {
        self.words
            .get(id / 64)
            .is_some_and(|word| word & (1u64 << (id % 64)) != 0)
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.words
            .iter()
            .map(|word| word.count_ones() as usize)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_ids_across_word_boundaries() {
        let mut set = WorldIdSet::with_capacity(130);
        for id in [0, 63, 64, 129] {
            set.insert(id);
        }

        assert_eq!(set.len(), 4);
        for id in [0, 63, 64, 129] {
            assert!(set.contains(id));
        }
        assert!(!set.contains(65));
    }
}
