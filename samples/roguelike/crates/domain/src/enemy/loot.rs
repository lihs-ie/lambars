use std::fmt;

use lambars::persistent::PersistentVector;

use crate::common::ValidationError;
use crate::item::ItemIdentifier;

// =============================================================================
// LootEntry
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LootEntry {
    item_identifier: ItemIdentifier,
    drop_rate: f32,
    min_quantity: u32,
    max_quantity: u32,
}

impl LootEntry {
    pub fn new(
        item_identifier: ItemIdentifier,
        drop_rate: f32,
        min_quantity: u32,
        max_quantity: u32,
    ) -> Result<Self, ValidationError> {
        if !(0.0..=1.0).contains(&drop_rate) {
            return Err(ValidationError::out_of_range(
                "drop_rate",
                0.0,
                1.0,
                drop_rate,
            ));
        }

        if min_quantity == 0 {
            return Err(ValidationError::constraint_violation(
                "min_quantity",
                "must be at least 1",
            ));
        }

        if min_quantity > max_quantity {
            return Err(ValidationError::constraint_violation(
                "min_quantity",
                "must not exceed max_quantity",
            ));
        }

        Ok(Self {
            item_identifier,
            drop_rate,
            min_quantity,
            max_quantity,
        })
    }

    #[must_use]
    pub const fn item_identifier(&self) -> ItemIdentifier {
        self.item_identifier
    }

    #[must_use]
    pub const fn drop_rate(&self) -> f32 {
        self.drop_rate
    }

    #[must_use]
    pub const fn min_quantity(&self) -> u32 {
        self.min_quantity
    }

    #[must_use]
    pub const fn max_quantity(&self) -> u32 {
        self.max_quantity
    }

    #[must_use]
    pub fn is_guaranteed(&self) -> bool {
        (self.drop_rate - 1.0).abs() < f32::EPSILON
    }

    #[must_use]
    pub const fn has_fixed_quantity(&self) -> bool {
        self.min_quantity == self.max_quantity
    }

    pub fn with_drop_rate(&self, drop_rate: f32) -> Result<Self, ValidationError> {
        Self::new(
            self.item_identifier,
            drop_rate,
            self.min_quantity,
            self.max_quantity,
        )
    }
}

impl fmt::Display for LootEntry {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.has_fixed_quantity() {
            write!(
                formatter,
                "{} x{} ({:.0}%)",
                self.item_identifier,
                self.min_quantity,
                self.drop_rate * 100.0
            )
        } else {
            write!(
                formatter,
                "{} x{}-{} ({:.0}%)",
                self.item_identifier,
                self.min_quantity,
                self.max_quantity,
                self.drop_rate * 100.0
            )
        }
    }
}

// =============================================================================
// LootTable
// =============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct LootTable {
    entries: PersistentVector<LootEntry>,
}

impl LootTable {
    #[must_use]
    pub const fn new(entries: PersistentVector<LootEntry>) -> Self {
        Self { entries }
    }

    #[must_use]
    pub fn empty() -> Self {
        Self {
            entries: PersistentVector::new(),
        }
    }

    #[must_use]
    pub const fn entries(&self) -> &PersistentVector<LootEntry> {
        &self.entries
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[must_use]
    pub fn with_entry(&self, entry: LootEntry) -> Self {
        Self {
            entries: self.entries.push_back(entry),
        }
    }

    #[must_use]
    pub fn merge(&self, other: &Self) -> Self {
        let mut entries = self.entries.clone();
        for entry in other.entries.iter() {
            entries = entries.push_back(*entry);
        }
        Self { entries }
    }

    pub fn iter(&self) -> impl Iterator<Item = &LootEntry> {
        self.entries.iter()
    }

    #[must_use]
    pub fn total_drop_rate(&self) -> f32 {
        self.entries
            .iter()
            .map(|entry: &LootEntry| entry.drop_rate())
            .sum()
    }
}

impl Default for LootTable {
    fn default() -> Self {
        Self::empty()
    }
}

impl fmt::Display for LootTable {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            write!(formatter, "LootTable (empty)")
        } else {
            write!(formatter, "LootTable ({} entries)", self.len())
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // LootEntry Tests
    // =========================================================================

    mod loot_entry {
        use super::*;

        // ---------------------------------------------------------------------
        // Construction Tests
        // ---------------------------------------------------------------------

        mod construction {
            use super::*;

            #[rstest]
            fn new_valid_entry() {
                let item_identifier = ItemIdentifier::new();
                let entry = LootEntry::new(item_identifier, 0.5, 1, 3).unwrap();

                assert_eq!(entry.item_identifier(), item_identifier);
                assert_eq!(entry.drop_rate(), 0.5);
                assert_eq!(entry.min_quantity(), 1);
                assert_eq!(entry.max_quantity(), 3);
            }

            #[rstest]
            fn new_with_zero_drop_rate() {
                let item_identifier = ItemIdentifier::new();
                let entry = LootEntry::new(item_identifier, 0.0, 1, 1).unwrap();
                assert_eq!(entry.drop_rate(), 0.0);
            }

            #[rstest]
            fn new_with_full_drop_rate() {
                let item_identifier = ItemIdentifier::new();
                let entry = LootEntry::new(item_identifier, 1.0, 1, 1).unwrap();
                assert_eq!(entry.drop_rate(), 1.0);
            }

            #[rstest]
            fn new_with_fixed_quantity() {
                let item_identifier = ItemIdentifier::new();
                let entry = LootEntry::new(item_identifier, 0.5, 5, 5).unwrap();
                assert_eq!(entry.min_quantity(), 5);
                assert_eq!(entry.max_quantity(), 5);
            }

            #[rstest]
            fn new_fails_with_negative_drop_rate() {
                let item_identifier = ItemIdentifier::new();
                let result = LootEntry::new(item_identifier, -0.1, 1, 1);
                assert!(result.is_err());
            }

            #[rstest]
            fn new_fails_with_drop_rate_above_one() {
                let item_identifier = ItemIdentifier::new();
                let result = LootEntry::new(item_identifier, 1.1, 1, 1);
                assert!(result.is_err());
            }

            #[rstest]
            fn new_fails_with_zero_min_quantity() {
                let item_identifier = ItemIdentifier::new();
                let result = LootEntry::new(item_identifier, 0.5, 0, 1);
                assert!(result.is_err());
            }

            #[rstest]
            fn new_fails_with_min_greater_than_max() {
                let item_identifier = ItemIdentifier::new();
                let result = LootEntry::new(item_identifier, 0.5, 5, 3);
                assert!(result.is_err());
            }
        }

        // ---------------------------------------------------------------------
        // Predicate Tests
        // ---------------------------------------------------------------------

        mod predicates {
            use super::*;

            #[rstest]
            fn is_guaranteed_true() {
                let item_identifier = ItemIdentifier::new();
                let entry = LootEntry::new(item_identifier, 1.0, 1, 1).unwrap();
                assert!(entry.is_guaranteed());
            }

            #[rstest]
            fn is_guaranteed_false() {
                let item_identifier = ItemIdentifier::new();
                let entry = LootEntry::new(item_identifier, 0.99, 1, 1).unwrap();
                assert!(!entry.is_guaranteed());
            }

            #[rstest]
            fn has_fixed_quantity_true() {
                let item_identifier = ItemIdentifier::new();
                let entry = LootEntry::new(item_identifier, 0.5, 3, 3).unwrap();
                assert!(entry.has_fixed_quantity());
            }

            #[rstest]
            fn has_fixed_quantity_false() {
                let item_identifier = ItemIdentifier::new();
                let entry = LootEntry::new(item_identifier, 0.5, 1, 3).unwrap();
                assert!(!entry.has_fixed_quantity());
            }
        }

        // ---------------------------------------------------------------------
        // Modification Tests
        // ---------------------------------------------------------------------

        mod modification {
            use super::*;

            #[rstest]
            fn with_drop_rate_valid() {
                let item_identifier = ItemIdentifier::new();
                let entry = LootEntry::new(item_identifier, 0.5, 1, 1).unwrap();
                let updated = entry.with_drop_rate(0.8).unwrap();

                assert_eq!(updated.drop_rate(), 0.8);
                assert_eq!(updated.item_identifier(), entry.item_identifier());
                assert_eq!(updated.min_quantity(), entry.min_quantity());
                assert_eq!(updated.max_quantity(), entry.max_quantity());
            }

            #[rstest]
            fn with_drop_rate_invalid() {
                let item_identifier = ItemIdentifier::new();
                let entry = LootEntry::new(item_identifier, 0.5, 1, 1).unwrap();
                let result = entry.with_drop_rate(1.5);
                assert!(result.is_err());
            }
        }

        // ---------------------------------------------------------------------
        // Display Tests
        // ---------------------------------------------------------------------

        mod display {
            use super::*;

            #[rstest]
            fn display_fixed_quantity() {
                let item_identifier = ItemIdentifier::new();
                let entry = LootEntry::new(item_identifier, 0.5, 3, 3).unwrap();
                let display = format!("{}", entry);

                assert!(display.contains("x3"));
                assert!(display.contains("50%"));
            }

            #[rstest]
            fn display_variable_quantity() {
                let item_identifier = ItemIdentifier::new();
                let entry = LootEntry::new(item_identifier, 0.25, 1, 5).unwrap();
                let display = format!("{}", entry);

                assert!(display.contains("x1-5"));
                assert!(display.contains("25%"));
            }
        }

        // ---------------------------------------------------------------------
        // Clone and Copy Tests
        // ---------------------------------------------------------------------

        mod clone_and_copy {
            use super::*;

            #[rstest]
            fn copy_preserves_values() {
                let item_identifier = ItemIdentifier::new();
                let entry = LootEntry::new(item_identifier, 0.5, 1, 3).unwrap();
                let copied: LootEntry = entry;

                assert_eq!(entry.item_identifier(), copied.item_identifier());
                assert_eq!(entry.drop_rate(), copied.drop_rate());
                assert_eq!(entry.min_quantity(), copied.min_quantity());
                assert_eq!(entry.max_quantity(), copied.max_quantity());
            }
        }
    }

    // =========================================================================
    // LootTable Tests
    // =========================================================================

    mod loot_table {
        use super::*;

        // ---------------------------------------------------------------------
        // Construction Tests
        // ---------------------------------------------------------------------

        mod construction {
            use super::*;

            #[rstest]
            fn empty_creates_empty_table() {
                let table = LootTable::empty();
                assert!(table.is_empty());
                assert_eq!(table.len(), 0);
            }

            #[rstest]
            fn new_creates_table_with_entries() {
                let item_identifier = ItemIdentifier::new();
                let entry = LootEntry::new(item_identifier, 0.5, 1, 1).unwrap();
                let entries = PersistentVector::from_iter([entry]);
                let table = LootTable::new(entries);

                assert!(!table.is_empty());
                assert_eq!(table.len(), 1);
            }

            #[rstest]
            fn default_creates_empty_table() {
                let table = LootTable::default();
                assert!(table.is_empty());
            }
        }

        // ---------------------------------------------------------------------
        // Modification Tests
        // ---------------------------------------------------------------------

        mod modification {
            use super::*;

            #[rstest]
            fn with_entry_adds_entry() {
                let table = LootTable::empty();
                let item_identifier = ItemIdentifier::new();
                let entry = LootEntry::new(item_identifier, 0.5, 1, 1).unwrap();

                let new_table = table.with_entry(entry);

                assert_eq!(new_table.len(), 1);
                assert!(table.is_empty()); // Original unchanged
            }

            #[rstest]
            fn with_entry_preserves_existing_entries() {
                let item1 = ItemIdentifier::new();
                let item2 = ItemIdentifier::new();
                let entry1 = LootEntry::new(item1, 0.5, 1, 1).unwrap();
                let entry2 = LootEntry::new(item2, 0.3, 1, 1).unwrap();

                let table = LootTable::empty().with_entry(entry1).with_entry(entry2);

                assert_eq!(table.len(), 2);
            }

            #[rstest]
            fn merge_combines_tables() {
                let item1 = ItemIdentifier::new();
                let item2 = ItemIdentifier::new();
                let entry1 = LootEntry::new(item1, 0.5, 1, 1).unwrap();
                let entry2 = LootEntry::new(item2, 0.3, 1, 1).unwrap();

                let table1 = LootTable::empty().with_entry(entry1);
                let table2 = LootTable::empty().with_entry(entry2);
                let merged = table1.merge(&table2);

                assert_eq!(merged.len(), 2);
            }

            #[rstest]
            fn merge_with_empty_table() {
                let item_identifier = ItemIdentifier::new();
                let entry = LootEntry::new(item_identifier, 0.5, 1, 1).unwrap();
                let table = LootTable::empty().with_entry(entry);
                let empty_table = LootTable::empty();

                let merged = table.merge(&empty_table);

                assert_eq!(merged.len(), 1);
            }
        }

        // ---------------------------------------------------------------------
        // Iteration Tests
        // ---------------------------------------------------------------------

        mod iteration {
            use super::*;

            #[rstest]
            fn iter_returns_all_entries() {
                let item1 = ItemIdentifier::new();
                let item2 = ItemIdentifier::new();
                let entry1 = LootEntry::new(item1, 0.5, 1, 1).unwrap();
                let entry2 = LootEntry::new(item2, 0.3, 1, 1).unwrap();

                let table = LootTable::empty().with_entry(entry1).with_entry(entry2);

                let count = table.iter().count();
                assert_eq!(count, 2);
            }

            #[rstest]
            fn iter_empty_table() {
                let table = LootTable::empty();
                let count = table.iter().count();
                assert_eq!(count, 0);
            }
        }

        // ---------------------------------------------------------------------
        // Total Drop Rate Tests
        // ---------------------------------------------------------------------

        mod total_drop_rate {
            use super::*;

            #[rstest]
            fn total_drop_rate_empty_table() {
                let table = LootTable::empty();
                assert_eq!(table.total_drop_rate(), 0.0);
            }

            #[rstest]
            fn total_drop_rate_single_entry() {
                let item_identifier = ItemIdentifier::new();
                let entry = LootEntry::new(item_identifier, 0.5, 1, 1).unwrap();
                let table = LootTable::empty().with_entry(entry);

                assert!((table.total_drop_rate() - 0.5).abs() < 0.01);
            }

            #[rstest]
            fn total_drop_rate_multiple_entries() {
                let item1 = ItemIdentifier::new();
                let item2 = ItemIdentifier::new();
                let item3 = ItemIdentifier::new();
                let entry1 = LootEntry::new(item1, 0.5, 1, 1).unwrap();
                let entry2 = LootEntry::new(item2, 0.3, 1, 1).unwrap();
                let entry3 = LootEntry::new(item3, 0.2, 1, 1).unwrap();

                let table = LootTable::empty()
                    .with_entry(entry1)
                    .with_entry(entry2)
                    .with_entry(entry3);

                assert!((table.total_drop_rate() - 1.0).abs() < 0.01);
            }
        }

        // ---------------------------------------------------------------------
        // Display Tests
        // ---------------------------------------------------------------------

        mod display {
            use super::*;

            #[rstest]
            fn display_empty_table() {
                let table = LootTable::empty();
                let display = format!("{}", table);
                assert!(display.contains("empty"));
            }

            #[rstest]
            fn display_table_with_entries() {
                let item_identifier = ItemIdentifier::new();
                let entry = LootEntry::new(item_identifier, 0.5, 1, 1).unwrap();
                let table = LootTable::empty().with_entry(entry);

                let display = format!("{}", table);
                assert!(display.contains("1 entries"));
            }
        }

        // ---------------------------------------------------------------------
        // Clone Tests
        // ---------------------------------------------------------------------

        mod clone {
            use super::*;

            #[rstest]
            fn clone_preserves_entries() {
                let item_identifier = ItemIdentifier::new();
                let entry = LootEntry::new(item_identifier, 0.5, 1, 1).unwrap();
                let table = LootTable::empty().with_entry(entry);
                let cloned = table.clone();

                assert_eq!(table.len(), cloned.len());
            }
        }
    }
}
