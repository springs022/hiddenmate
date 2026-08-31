use std::collections::BTreeSet;

use fmrs_core::piece::{Kind, KINDS};

/// 14種類の駒種候補を保持する小さなビット集合。
///
/// 公開問題形式では従来どおり`Vec<Kind>`を受け取り、探索コアへ入る時点で
/// この表現へ正規化する。重複除去と候補走査で木構造を割り当てない。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct KindSet(u16);

impl KindSet {
    pub(crate) fn from_iter(kinds: impl IntoIterator<Item = Kind>) -> Self {
        let mut result = Self::default();
        for kind in kinds {
            result.insert(kind);
        }
        result
    }

    pub(crate) fn insert(&mut self, kind: Kind) {
        self.0 |= 1 << kind.index();
    }

    pub(crate) fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub(crate) fn len(self) -> usize {
        self.0.count_ones() as usize
    }

    pub(crate) fn iter(self) -> impl Iterator<Item = Kind> {
        KINDS.into_iter().filter(move |kind| self.contains(*kind))
    }

    pub(crate) fn to_btree_set(self) -> BTreeSet<Kind> {
        self.iter().collect()
    }

    fn contains(self, kind: Kind) -> bool {
        self.0 & (1 << kind.index()) != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deduplicates_and_iterates_in_kind_order() {
        let set = KindSet::from_iter([Kind::Rook, Kind::Pawn, Kind::Rook]);

        assert_eq!(set.len(), 2);
        assert_eq!(set.iter().collect::<Vec<_>>(), vec![Kind::Pawn, Kind::Rook]);
    }
}
