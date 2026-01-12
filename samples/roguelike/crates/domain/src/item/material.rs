
use std::fmt;

use crate::common::Rarity;

// =============================================================================
// MaterialData
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MaterialData {
    rarity: Rarity,
    max_stack: u32,
}

impl MaterialData {
    #[must_use]
    pub const fn new(rarity: Rarity, max_stack: u32) -> Self {
        Self { rarity, max_stack }
    }

    #[must_use]
    pub const fn rarity(&self) -> Rarity {
        self.rarity
    }

    #[must_use]
    pub const fn max_stack(&self) -> u32 {
        self.max_stack
    }

    #[must_use]
    pub fn value_multiplier(&self) -> f32 {
        match self.rarity {
            Rarity::Common => 1.0,
            Rarity::Uncommon => 2.5,
            Rarity::Rare => 5.0,
            Rarity::Epic => 10.0,
            Rarity::Legendary => 25.0,
        }
    }

    #[must_use]
    pub const fn with_max_stack(&self, max_stack: u32) -> Self {
        Self {
            rarity: self.rarity,
            max_stack,
        }
    }

    #[must_use]
    pub const fn with_rarity(&self, rarity: Rarity) -> Self {
        Self {
            rarity,
            max_stack: self.max_stack,
        }
    }
}

impl fmt::Display for MaterialData {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} Material (max stack: {})",
            self.rarity, self.max_stack
        )
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    fn create_material_data() -> MaterialData {
        MaterialData::new(Rarity::Rare, 20)
    }

    #[rstest]
    fn new_creates_material_data() {
        let material = create_material_data();
        assert_eq!(material.rarity(), Rarity::Rare);
        assert_eq!(material.max_stack(), 20);
    }

    #[rstest]
    fn new_with_common_rarity() {
        let material = MaterialData::new(Rarity::Common, 99);
        assert_eq!(material.rarity(), Rarity::Common);
    }

    #[rstest]
    fn new_with_legendary_rarity() {
        let material = MaterialData::new(Rarity::Legendary, 10);
        assert_eq!(material.rarity(), Rarity::Legendary);
    }

    #[rstest]
    fn new_with_zero_stack() {
        let material = MaterialData::new(Rarity::Uncommon, 0);
        assert_eq!(material.max_stack(), 0);
    }

    #[rstest]
    #[case(Rarity::Common, 1.0)]
    #[case(Rarity::Uncommon, 2.5)]
    #[case(Rarity::Rare, 5.0)]
    #[case(Rarity::Epic, 10.0)]
    #[case(Rarity::Legendary, 25.0)]
    fn value_multiplier(#[case] rarity: Rarity, #[case] expected: f32) {
        let material = MaterialData::new(rarity, 10);
        assert!((material.value_multiplier() - expected).abs() < 1e-6);
    }

    #[rstest]
    fn value_multiplier_increases_with_rarity() {
        let rarities = Rarity::all();
        for i in 0..rarities.len() - 1 {
            let lower = MaterialData::new(rarities[i], 10);
            let higher = MaterialData::new(rarities[i + 1], 10);
            assert!(
                lower.value_multiplier() < higher.value_multiplier(),
                "{:?} should have lower multiplier than {:?}",
                rarities[i],
                rarities[i + 1]
            );
        }
    }

    #[rstest]
    fn with_max_stack_changes_stack() {
        let material = create_material_data();
        let modified = material.with_max_stack(50);

        assert_eq!(modified.max_stack(), 50);
        assert_eq!(modified.rarity(), Rarity::Rare);
    }

    #[rstest]
    fn with_rarity_changes_rarity() {
        let material = create_material_data();
        let modified = material.with_rarity(Rarity::Epic);

        assert_eq!(modified.rarity(), Rarity::Epic);
        assert_eq!(modified.max_stack(), 20);
    }

    #[rstest]
    fn display_format() {
        let material = create_material_data();
        assert_eq!(format!("{}", material), "Rare Material (max stack: 20)");
    }

    #[rstest]
    fn display_format_common() {
        let material = MaterialData::new(Rarity::Common, 99);
        assert_eq!(format!("{}", material), "Common Material (max stack: 99)");
    }

    #[rstest]
    fn display_format_legendary() {
        let material = MaterialData::new(Rarity::Legendary, 5);
        assert_eq!(
            format!("{}", material),
            "Legendary Material (max stack: 5)"
        );
    }

    #[rstest]
    fn equality() {
        let material1 = create_material_data();
        let material2 = create_material_data();
        let material3 = MaterialData::new(Rarity::Epic, 20);

        assert_eq!(material1, material2);
        assert_ne!(material1, material3);
    }

    #[rstest]
    fn equality_different_stack() {
        let material1 = create_material_data();
        let material2 = MaterialData::new(Rarity::Rare, 30);

        assert_ne!(material1, material2);
    }

    #[rstest]
    fn clone() {
        let material = create_material_data();
        let cloned = material;
        assert_eq!(material, cloned);
    }

    #[rstest]
    fn hash_consistency() {
        use std::collections::HashSet;

        let material1 = create_material_data();
        let material2 = create_material_data();
        let material3 = MaterialData::new(Rarity::Common, 99);

        let mut set = HashSet::new();
        set.insert(material1);

        assert!(set.contains(&material2));
        assert!(!set.contains(&material3));
    }

    #[rstest]
    fn debug_format() {
        let material = create_material_data();
        let debug_string = format!("{:?}", material);
        assert!(debug_string.contains("MaterialData"));
        assert!(debug_string.contains("rarity"));
    }
}
