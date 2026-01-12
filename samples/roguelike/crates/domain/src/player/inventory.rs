//! Player equipment and inventory types.
//!
//! This module provides types for managing player equipment and inventory:
//!
//! - **EquipmentSlot**: Enum representing equipment slot types
//! - **EquipmentSlots**: Struct holding all equipment slots
//! - **Inventory**: Container for player's item stacks
//! - **ItemStack**: A stack of items with quantity

use std::fmt;

use crate::item::ItemIdentifier;
use crate::player::PlayerError;

// =============================================================================
// EquipmentSlot
// =============================================================================

/// Represents the type of equipment slot.
///
/// Each slot type can hold a specific category of equipment item.
///
/// # Examples
///
/// ```
/// use roguelike_domain::player::EquipmentSlot;
///
/// let slot = EquipmentSlot::Weapon;
/// println!("Equipping to: {}", slot);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EquipmentSlot {
    /// Weapon slot for swords, staffs, bows, etc.
    Weapon,
    /// Armor slot for body armor.
    Armor,
    /// Helmet slot for head protection.
    Helmet,
    /// Accessory slot for rings, amulets, etc.
    Accessory,
}

impl EquipmentSlot {
    /// Returns all equipment slot types.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::player::EquipmentSlot;
    ///
    /// let slots = EquipmentSlot::all();
    /// assert_eq!(slots.len(), 4);
    /// ```
    #[must_use]
    pub const fn all() -> [Self; 4] {
        [Self::Weapon, Self::Armor, Self::Helmet, Self::Accessory]
    }

    /// Returns the slot name as a string.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::player::EquipmentSlot;
    ///
    /// assert_eq!(EquipmentSlot::Weapon.name(), "Weapon");
    /// ```
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Weapon => "Weapon",
            Self::Armor => "Armor",
            Self::Helmet => "Helmet",
            Self::Accessory => "Accessory",
        }
    }
}

impl fmt::Display for EquipmentSlot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.name())
    }
}

// =============================================================================
// EquipmentSlots
// =============================================================================

/// Container for all equipment slots.
///
/// Each slot can optionally hold an item identifier.
/// Note: Currently using placeholder type (`Option<String>`) as `Item` is not yet implemented.
///
/// # Examples
///
/// ```
/// use roguelike_domain::player::{EquipmentSlots, EquipmentSlot};
///
/// let equipment = EquipmentSlots::empty();
/// assert!(equipment.is_slot_empty(EquipmentSlot::Weapon));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EquipmentSlots {
    /// The weapon slot.
    weapon: Option<String>, // TODO: Replace with Option<Item> when Item is implemented
    /// The armor slot.
    armor: Option<String>, // TODO: Replace with Option<Item> when Item is implemented
    /// The helmet slot.
    helmet: Option<String>, // TODO: Replace with Option<Item> when Item is implemented
    /// The accessory slot.
    accessory: Option<String>, // TODO: Replace with Option<Item> when Item is implemented
}

impl EquipmentSlots {
    /// Creates empty equipment slots.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::player::EquipmentSlots;
    ///
    /// let equipment = EquipmentSlots::empty();
    /// ```
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            weapon: None,
            armor: None,
            helmet: None,
            accessory: None,
        }
    }

    /// Returns a reference to the weapon slot.
    #[must_use]
    pub fn weapon(&self) -> Option<&str> {
        self.weapon.as_deref()
    }

    /// Returns a reference to the armor slot.
    #[must_use]
    pub fn armor(&self) -> Option<&str> {
        self.armor.as_deref()
    }

    /// Returns a reference to the helmet slot.
    #[must_use]
    pub fn helmet(&self) -> Option<&str> {
        self.helmet.as_deref()
    }

    /// Returns a reference to the accessory slot.
    #[must_use]
    pub fn accessory(&self) -> Option<&str> {
        self.accessory.as_deref()
    }

    /// Returns the item in the specified slot.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::player::{EquipmentSlots, EquipmentSlot};
    ///
    /// let equipment = EquipmentSlots::empty();
    /// assert!(equipment.get(EquipmentSlot::Weapon).is_none());
    /// ```
    #[must_use]
    pub fn get(&self, slot: EquipmentSlot) -> Option<&str> {
        match slot {
            EquipmentSlot::Weapon => self.weapon(),
            EquipmentSlot::Armor => self.armor(),
            EquipmentSlot::Helmet => self.helmet(),
            EquipmentSlot::Accessory => self.accessory(),
        }
    }

    /// Checks if the specified slot is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::player::{EquipmentSlots, EquipmentSlot};
    ///
    /// let equipment = EquipmentSlots::empty();
    /// assert!(equipment.is_slot_empty(EquipmentSlot::Weapon));
    /// ```
    #[must_use]
    pub fn is_slot_empty(&self, slot: EquipmentSlot) -> bool {
        self.get(slot).is_none()
    }

    /// Checks if the specified slot is occupied.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::player::{EquipmentSlots, EquipmentSlot};
    ///
    /// let equipment = EquipmentSlots::empty();
    /// assert!(!equipment.is_slot_occupied(EquipmentSlot::Weapon));
    /// ```
    #[must_use]
    pub fn is_slot_occupied(&self, slot: EquipmentSlot) -> bool {
        self.get(slot).is_some()
    }

    /// Equips an item to the specified slot, returning new `EquipmentSlots`.
    ///
    /// This is an immutable operation that returns a new instance.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::player::{EquipmentSlots, EquipmentSlot};
    ///
    /// let equipment = EquipmentSlots::empty();
    /// let equipped = equipment.equip(EquipmentSlot::Weapon, "sword_01".to_string());
    /// assert!(equipped.is_slot_occupied(EquipmentSlot::Weapon));
    /// ```
    #[must_use]
    pub fn equip(&self, slot: EquipmentSlot, item: String) -> Self {
        let mut new_slots = self.clone();
        match slot {
            EquipmentSlot::Weapon => new_slots.weapon = Some(item),
            EquipmentSlot::Armor => new_slots.armor = Some(item),
            EquipmentSlot::Helmet => new_slots.helmet = Some(item),
            EquipmentSlot::Accessory => new_slots.accessory = Some(item),
        }
        new_slots
    }

    /// Unequips an item from the specified slot, returning new `EquipmentSlots`.
    ///
    /// This is an immutable operation that returns a new instance.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::player::{EquipmentSlots, EquipmentSlot};
    ///
    /// let equipment = EquipmentSlots::empty()
    ///     .equip(EquipmentSlot::Weapon, "sword_01".to_string());
    /// let unequipped = equipment.unequip(EquipmentSlot::Weapon);
    /// assert!(unequipped.is_slot_empty(EquipmentSlot::Weapon));
    /// ```
    #[must_use]
    pub fn unequip(&self, slot: EquipmentSlot) -> Self {
        let mut new_slots = self.clone();
        match slot {
            EquipmentSlot::Weapon => new_slots.weapon = None,
            EquipmentSlot::Armor => new_slots.armor = None,
            EquipmentSlot::Helmet => new_slots.helmet = None,
            EquipmentSlot::Accessory => new_slots.accessory = None,
        }
        new_slots
    }

    /// Returns the number of equipped items.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::player::{EquipmentSlots, EquipmentSlot};
    ///
    /// let equipment = EquipmentSlots::empty()
    ///     .equip(EquipmentSlot::Weapon, "sword".to_string())
    ///     .equip(EquipmentSlot::Armor, "plate".to_string());
    /// assert_eq!(equipment.equipped_count(), 2);
    /// ```
    #[must_use]
    pub fn equipped_count(&self) -> usize {
        EquipmentSlot::all()
            .iter()
            .filter(|slot| self.is_slot_occupied(**slot))
            .count()
    }

    /// Returns true if all slots are empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::player::EquipmentSlots;
    ///
    /// let equipment = EquipmentSlots::empty();
    /// assert!(equipment.is_all_empty());
    /// ```
    #[must_use]
    pub fn is_all_empty(&self) -> bool {
        self.equipped_count() == 0
    }

    /// Returns true if all slots are occupied.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::player::{EquipmentSlots, EquipmentSlot};
    ///
    /// let equipment = EquipmentSlots::empty()
    ///     .equip(EquipmentSlot::Weapon, "sword".to_string())
    ///     .equip(EquipmentSlot::Armor, "plate".to_string())
    ///     .equip(EquipmentSlot::Helmet, "helm".to_string())
    ///     .equip(EquipmentSlot::Accessory, "ring".to_string());
    /// assert!(equipment.is_fully_equipped());
    /// ```
    #[must_use]
    pub fn is_fully_equipped(&self) -> bool {
        self.equipped_count() == EquipmentSlot::all().len()
    }
}

// =============================================================================
// ItemStack
// =============================================================================

/// A stack of items with an identifier and quantity.
///
/// `ItemStack` represents one or more identical items in the player's inventory.
///
/// # Examples
///
/// ```
/// use roguelike_domain::item::ItemIdentifier;
/// use roguelike_domain::player::ItemStack;
///
/// let item_id = ItemIdentifier::new();
/// let stack = ItemStack::new(item_id, 5);
/// assert_eq!(stack.quantity(), 5);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ItemStack {
    item_identifier: ItemIdentifier,
    quantity: u32,
}

impl ItemStack {
    /// Creates a new `ItemStack` with the given item identifier and quantity.
    ///
    /// # Arguments
    ///
    /// * `item_identifier` - The unique identifier for this item type
    /// * `quantity` - The number of items in this stack
    #[must_use]
    pub const fn new(item_identifier: ItemIdentifier, quantity: u32) -> Self {
        Self {
            item_identifier,
            quantity,
        }
    }

    /// Returns the item identifier.
    #[must_use]
    pub const fn item_identifier(&self) -> ItemIdentifier {
        self.item_identifier
    }

    /// Returns the quantity of items in this stack.
    #[must_use]
    pub const fn quantity(&self) -> u32 {
        self.quantity
    }

    /// Returns a new `ItemStack` with the quantity increased by the given amount.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::item::ItemIdentifier;
    /// use roguelike_domain::player::ItemStack;
    ///
    /// let item_id = ItemIdentifier::new();
    /// let stack = ItemStack::new(item_id, 5);
    /// let increased = stack.add_quantity(3);
    /// assert_eq!(increased.quantity(), 8);
    /// ```
    #[must_use]
    pub const fn add_quantity(&self, amount: u32) -> Self {
        Self {
            item_identifier: self.item_identifier,
            quantity: self.quantity.saturating_add(amount),
        }
    }

    /// Returns a new `ItemStack` with the quantity decreased by the given amount.
    ///
    /// Returns `None` if the quantity would become zero or negative.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::item::ItemIdentifier;
    /// use roguelike_domain::player::ItemStack;
    ///
    /// let item_id = ItemIdentifier::new();
    /// let stack = ItemStack::new(item_id, 5);
    /// let decreased = stack.remove_quantity(3).unwrap();
    /// assert_eq!(decreased.quantity(), 2);
    ///
    /// // Removing all items returns None
    /// let empty = stack.remove_quantity(5);
    /// assert!(empty.is_none());
    /// ```
    #[must_use]
    pub const fn remove_quantity(&self, amount: u32) -> Option<Self> {
        if amount >= self.quantity {
            None
        } else {
            Some(Self {
                item_identifier: self.item_identifier,
                quantity: self.quantity - amount,
            })
        }
    }
}

impl fmt::Display for ItemStack {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} x{}", self.item_identifier, self.quantity)
    }
}

// =============================================================================
// Inventory
// =============================================================================

/// Default inventory capacity.
const DEFAULT_INVENTORY_CAPACITY: u32 = 20;

/// Container for player's item stacks.
///
/// `Inventory` holds multiple `ItemStack`s with a maximum capacity.
/// Items with the same identifier are automatically stacked together.
///
/// # Invariants
///
/// - `items.len() <= capacity`
///
/// # Examples
///
/// ```
/// use roguelike_domain::item::ItemIdentifier;
/// use roguelike_domain::player::Inventory;
///
/// let inventory = Inventory::new(10);
/// assert!(inventory.is_empty());
/// assert_eq!(inventory.capacity(), 10);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inventory {
    items: Vec<ItemStack>,
    capacity: u32,
}

impl Inventory {
    /// Creates a new empty `Inventory` with the given capacity.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::player::Inventory;
    ///
    /// let inventory = Inventory::new(20);
    /// assert_eq!(inventory.capacity(), 20);
    /// ```
    #[must_use]
    pub fn new(capacity: u32) -> Self {
        Self {
            items: Vec::new(),
            capacity,
        }
    }

    /// Creates a new empty `Inventory` with the specified capacity.
    ///
    /// This is an alias for `new` to match common Rust naming conventions.
    #[must_use]
    pub fn with_capacity(capacity: u32) -> Self {
        Self::new(capacity)
    }

    /// Returns the maximum number of different item stacks this inventory can hold.
    #[must_use]
    pub const fn capacity(&self) -> u32 {
        self.capacity
    }

    /// Returns the number of different item stacks in the inventory.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns true if the inventory has no items.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Returns true if the inventory is at maximum capacity.
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.items.len() >= self.capacity as usize
    }

    /// Returns a slice of all item stacks in the inventory.
    #[must_use]
    pub fn items(&self) -> &[ItemStack] {
        &self.items
    }

    /// Finds an item stack by its identifier.
    #[must_use]
    pub fn find(&self, item_identifier: &ItemIdentifier) -> Option<&ItemStack> {
        self.items
            .iter()
            .find(|stack| &stack.item_identifier == item_identifier)
    }

    /// Returns the total quantity of items with the given identifier.
    #[must_use]
    pub fn quantity_of(&self, item_identifier: &ItemIdentifier) -> u32 {
        self.find(item_identifier)
            .map(|stack| stack.quantity())
            .unwrap_or(0)
    }

    /// Adds items to the inventory.
    ///
    /// If an item with the same identifier already exists, the quantities are combined.
    /// If the inventory is full and the item is new, returns an error.
    ///
    /// This is an immutable operation that returns a new `Inventory`.
    ///
    /// # Errors
    ///
    /// Returns `PlayerError::InventoryFull` if the inventory is at capacity
    /// and a new item type is being added.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::item::ItemIdentifier;
    /// use roguelike_domain::player::Inventory;
    ///
    /// let inventory = Inventory::new(10);
    /// let item_id = ItemIdentifier::new();
    /// let updated = inventory.add_item(item_id, 5).unwrap();
    /// assert_eq!(updated.len(), 1);
    /// ```
    pub fn add_item(
        self,
        item_identifier: ItemIdentifier,
        quantity: u32,
    ) -> Result<Self, PlayerError> {
        // Check if item already exists
        let existing_index = self
            .items
            .iter()
            .position(|stack| stack.item_identifier == item_identifier);

        match existing_index {
            Some(index) => {
                // Item exists, increase quantity
                let mut new_items = self.items;
                new_items[index] = new_items[index].add_quantity(quantity);
                Ok(Self {
                    items: new_items,
                    capacity: self.capacity,
                })
            }
            None => {
                // New item, check capacity
                if self.is_full() {
                    return Err(PlayerError::inventory_full(self.capacity));
                }
                let mut new_items = self.items;
                new_items.push(ItemStack::new(item_identifier, quantity));
                Ok(Self {
                    items: new_items,
                    capacity: self.capacity,
                })
            }
        }
    }

    /// Removes items from the inventory.
    ///
    /// If the quantity to remove equals the stack quantity, the stack is removed.
    /// If the quantity to remove is greater than available, returns an error.
    ///
    /// This is an immutable operation that returns a new `Inventory`.
    ///
    /// # Errors
    ///
    /// Returns `PlayerError::ItemNotInInventory` if the item is not found
    /// or if the quantity to remove exceeds the available quantity.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::item::ItemIdentifier;
    /// use roguelike_domain::player::Inventory;
    ///
    /// let item_id = ItemIdentifier::new();
    /// let inventory = Inventory::new(10)
    ///     .add_item(item_id, 5)
    ///     .unwrap();
    /// let updated = inventory.remove_item(&item_id, 3).unwrap();
    /// assert_eq!(updated.quantity_of(&item_id), 2);
    /// ```
    pub fn remove_item(
        self,
        item_identifier: &ItemIdentifier,
        quantity: u32,
    ) -> Result<Self, PlayerError> {
        let existing_index = self
            .items
            .iter()
            .position(|stack| &stack.item_identifier == item_identifier);

        match existing_index {
            Some(index) => {
                let stack = &self.items[index];
                if quantity > stack.quantity() {
                    return Err(PlayerError::item_not_in_inventory(
                        item_identifier.to_string(),
                    ));
                }

                let mut new_items = self.items;
                if let Some(new_stack) = new_items[index].remove_quantity(quantity) {
                    new_items[index] = new_stack;
                } else {
                    new_items.remove(index);
                }

                Ok(Self {
                    items: new_items,
                    capacity: self.capacity,
                })
            }
            None => Err(PlayerError::item_not_in_inventory(
                item_identifier.to_string(),
            )),
        }
    }
}

impl Default for Inventory {
    fn default() -> Self {
        Self::new(DEFAULT_INVENTORY_CAPACITY)
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
    // EquipmentSlot Tests
    // =========================================================================

    mod equipment_slot {
        use super::*;

        #[rstest]
        fn all_returns_four_slots() {
            let slots = EquipmentSlot::all();
            assert_eq!(slots.len(), 4);
        }

        #[rstest]
        fn all_contains_all_variants() {
            let slots = EquipmentSlot::all();
            assert!(slots.contains(&EquipmentSlot::Weapon));
            assert!(slots.contains(&EquipmentSlot::Armor));
            assert!(slots.contains(&EquipmentSlot::Helmet));
            assert!(slots.contains(&EquipmentSlot::Accessory));
        }

        #[rstest]
        #[case(EquipmentSlot::Weapon, "Weapon")]
        #[case(EquipmentSlot::Armor, "Armor")]
        #[case(EquipmentSlot::Helmet, "Helmet")]
        #[case(EquipmentSlot::Accessory, "Accessory")]
        fn name_returns_correct_string(#[case] slot: EquipmentSlot, #[case] expected: &str) {
            assert_eq!(slot.name(), expected);
        }

        #[rstest]
        #[case(EquipmentSlot::Weapon, "Weapon")]
        #[case(EquipmentSlot::Armor, "Armor")]
        #[case(EquipmentSlot::Helmet, "Helmet")]
        #[case(EquipmentSlot::Accessory, "Accessory")]
        fn display_format(#[case] slot: EquipmentSlot, #[case] expected: &str) {
            assert_eq!(format!("{}", slot), expected);
        }

        #[rstest]
        fn equality() {
            assert_eq!(EquipmentSlot::Weapon, EquipmentSlot::Weapon);
            assert_ne!(EquipmentSlot::Weapon, EquipmentSlot::Armor);
        }

        #[rstest]
        fn clone() {
            let slot = EquipmentSlot::Weapon;
            let cloned = slot;
            assert_eq!(slot, cloned);
        }

        #[rstest]
        fn hash_consistency() {
            use std::collections::HashSet;

            let mut set = HashSet::new();
            set.insert(EquipmentSlot::Weapon);

            assert!(set.contains(&EquipmentSlot::Weapon));
            assert!(!set.contains(&EquipmentSlot::Armor));
        }

        #[rstest]
        fn debug_format() {
            let slot = EquipmentSlot::Weapon;
            let debug_string = format!("{:?}", slot);
            assert!(debug_string.contains("Weapon"));
        }
    }

    // =========================================================================
    // EquipmentSlots Tests
    // =========================================================================

    mod equipment_slots {
        use super::*;

        #[rstest]
        fn empty_creates_empty_slots() {
            let equipment = EquipmentSlots::empty();
            assert!(equipment.is_all_empty());
        }

        #[rstest]
        fn default_creates_empty_slots() {
            let equipment = EquipmentSlots::default();
            assert!(equipment.is_all_empty());
        }

        #[rstest]
        fn weapon_returns_none_when_empty() {
            let equipment = EquipmentSlots::empty();
            assert!(equipment.weapon().is_none());
        }

        #[rstest]
        fn armor_returns_none_when_empty() {
            let equipment = EquipmentSlots::empty();
            assert!(equipment.armor().is_none());
        }

        #[rstest]
        fn helmet_returns_none_when_empty() {
            let equipment = EquipmentSlots::empty();
            assert!(equipment.helmet().is_none());
        }

        #[rstest]
        fn accessory_returns_none_when_empty() {
            let equipment = EquipmentSlots::empty();
            assert!(equipment.accessory().is_none());
        }

        #[rstest]
        fn get_returns_item_in_slot() {
            let equipment =
                EquipmentSlots::empty().equip(EquipmentSlot::Weapon, "sword".to_string());
            assert_eq!(equipment.get(EquipmentSlot::Weapon), Some("sword"));
        }

        #[rstest]
        fn get_returns_none_for_empty_slot() {
            let equipment = EquipmentSlots::empty();
            assert!(equipment.get(EquipmentSlot::Weapon).is_none());
        }

        #[rstest]
        fn is_slot_empty_returns_true_for_empty_slot() {
            let equipment = EquipmentSlots::empty();
            assert!(equipment.is_slot_empty(EquipmentSlot::Weapon));
        }

        #[rstest]
        fn is_slot_empty_returns_false_for_occupied_slot() {
            let equipment =
                EquipmentSlots::empty().equip(EquipmentSlot::Weapon, "sword".to_string());
            assert!(!equipment.is_slot_empty(EquipmentSlot::Weapon));
        }

        #[rstest]
        fn is_slot_occupied_returns_false_for_empty_slot() {
            let equipment = EquipmentSlots::empty();
            assert!(!equipment.is_slot_occupied(EquipmentSlot::Weapon));
        }

        #[rstest]
        fn is_slot_occupied_returns_true_for_occupied_slot() {
            let equipment =
                EquipmentSlots::empty().equip(EquipmentSlot::Weapon, "sword".to_string());
            assert!(equipment.is_slot_occupied(EquipmentSlot::Weapon));
        }

        #[rstest]
        fn equip_adds_item_to_slot() {
            let equipment = EquipmentSlots::empty();
            let equipped = equipment.equip(EquipmentSlot::Weapon, "sword".to_string());
            assert_eq!(equipped.weapon(), Some("sword"));
        }

        #[rstest]
        fn equip_to_armor_slot() {
            let equipment = EquipmentSlots::empty();
            let equipped = equipment.equip(EquipmentSlot::Armor, "plate".to_string());
            assert_eq!(equipped.armor(), Some("plate"));
        }

        #[rstest]
        fn equip_to_helmet_slot() {
            let equipment = EquipmentSlots::empty();
            let equipped = equipment.equip(EquipmentSlot::Helmet, "helm".to_string());
            assert_eq!(equipped.helmet(), Some("helm"));
        }

        #[rstest]
        fn equip_to_accessory_slot() {
            let equipment = EquipmentSlots::empty();
            let equipped = equipment.equip(EquipmentSlot::Accessory, "ring".to_string());
            assert_eq!(equipped.accessory(), Some("ring"));
        }

        #[rstest]
        fn equip_does_not_modify_original() {
            let original = EquipmentSlots::empty();
            let _ = original.equip(EquipmentSlot::Weapon, "sword".to_string());
            assert!(original.is_slot_empty(EquipmentSlot::Weapon));
        }

        #[rstest]
        fn equip_replaces_existing_item() {
            let equipment =
                EquipmentSlots::empty().equip(EquipmentSlot::Weapon, "old_sword".to_string());
            let replaced = equipment.equip(EquipmentSlot::Weapon, "new_sword".to_string());
            assert_eq!(replaced.weapon(), Some("new_sword"));
        }

        #[rstest]
        fn unequip_removes_item_from_slot() {
            let equipment =
                EquipmentSlots::empty().equip(EquipmentSlot::Weapon, "sword".to_string());
            let unequipped = equipment.unequip(EquipmentSlot::Weapon);
            assert!(unequipped.weapon().is_none());
        }

        #[rstest]
        fn unequip_from_armor_slot() {
            let equipment =
                EquipmentSlots::empty().equip(EquipmentSlot::Armor, "plate".to_string());
            let unequipped = equipment.unequip(EquipmentSlot::Armor);
            assert!(unequipped.armor().is_none());
        }

        #[rstest]
        fn unequip_from_helmet_slot() {
            let equipment =
                EquipmentSlots::empty().equip(EquipmentSlot::Helmet, "helm".to_string());
            let unequipped = equipment.unequip(EquipmentSlot::Helmet);
            assert!(unequipped.helmet().is_none());
        }

        #[rstest]
        fn unequip_from_accessory_slot() {
            let equipment =
                EquipmentSlots::empty().equip(EquipmentSlot::Accessory, "ring".to_string());
            let unequipped = equipment.unequip(EquipmentSlot::Accessory);
            assert!(unequipped.accessory().is_none());
        }

        #[rstest]
        fn unequip_does_not_modify_original() {
            let original =
                EquipmentSlots::empty().equip(EquipmentSlot::Weapon, "sword".to_string());
            let _ = original.unequip(EquipmentSlot::Weapon);
            assert!(original.is_slot_occupied(EquipmentSlot::Weapon));
        }

        #[rstest]
        fn unequip_from_empty_slot_returns_empty_slot() {
            let equipment = EquipmentSlots::empty();
            let unequipped = equipment.unequip(EquipmentSlot::Weapon);
            assert!(unequipped.is_slot_empty(EquipmentSlot::Weapon));
        }

        #[rstest]
        fn equipped_count_returns_zero_for_empty() {
            let equipment = EquipmentSlots::empty();
            assert_eq!(equipment.equipped_count(), 0);
        }

        #[rstest]
        fn equipped_count_returns_correct_count() {
            let equipment = EquipmentSlots::empty()
                .equip(EquipmentSlot::Weapon, "sword".to_string())
                .equip(EquipmentSlot::Armor, "plate".to_string());
            assert_eq!(equipment.equipped_count(), 2);
        }

        #[rstest]
        fn equipped_count_returns_four_when_fully_equipped() {
            let equipment = EquipmentSlots::empty()
                .equip(EquipmentSlot::Weapon, "sword".to_string())
                .equip(EquipmentSlot::Armor, "plate".to_string())
                .equip(EquipmentSlot::Helmet, "helm".to_string())
                .equip(EquipmentSlot::Accessory, "ring".to_string());
            assert_eq!(equipment.equipped_count(), 4);
        }

        #[rstest]
        fn is_all_empty_returns_true_for_empty() {
            let equipment = EquipmentSlots::empty();
            assert!(equipment.is_all_empty());
        }

        #[rstest]
        fn is_all_empty_returns_false_when_any_equipped() {
            let equipment =
                EquipmentSlots::empty().equip(EquipmentSlot::Weapon, "sword".to_string());
            assert!(!equipment.is_all_empty());
        }

        #[rstest]
        fn is_fully_equipped_returns_false_for_empty() {
            let equipment = EquipmentSlots::empty();
            assert!(!equipment.is_fully_equipped());
        }

        #[rstest]
        fn is_fully_equipped_returns_false_when_partial() {
            let equipment = EquipmentSlots::empty()
                .equip(EquipmentSlot::Weapon, "sword".to_string())
                .equip(EquipmentSlot::Armor, "plate".to_string());
            assert!(!equipment.is_fully_equipped());
        }

        #[rstest]
        fn is_fully_equipped_returns_true_when_all_slots_filled() {
            let equipment = EquipmentSlots::empty()
                .equip(EquipmentSlot::Weapon, "sword".to_string())
                .equip(EquipmentSlot::Armor, "plate".to_string())
                .equip(EquipmentSlot::Helmet, "helm".to_string())
                .equip(EquipmentSlot::Accessory, "ring".to_string());
            assert!(equipment.is_fully_equipped());
        }

        #[rstest]
        fn equality() {
            let equipment1 =
                EquipmentSlots::empty().equip(EquipmentSlot::Weapon, "sword".to_string());
            let equipment2 =
                EquipmentSlots::empty().equip(EquipmentSlot::Weapon, "sword".to_string());
            assert_eq!(equipment1, equipment2);
        }

        #[rstest]
        fn inequality() {
            let equipment1 =
                EquipmentSlots::empty().equip(EquipmentSlot::Weapon, "sword".to_string());
            let equipment2 =
                EquipmentSlots::empty().equip(EquipmentSlot::Weapon, "axe".to_string());
            assert_ne!(equipment1, equipment2);
        }

        #[rstest]
        fn clone() {
            let equipment =
                EquipmentSlots::empty().equip(EquipmentSlot::Weapon, "sword".to_string());
            let cloned = equipment.clone();
            assert_eq!(equipment, cloned);
        }

        #[rstest]
        fn debug_format() {
            let equipment = EquipmentSlots::empty();
            let debug_string = format!("{:?}", equipment);
            assert!(debug_string.contains("EquipmentSlots"));
        }

        #[rstest]
        fn chain_multiple_equips() {
            let equipment = EquipmentSlots::empty()
                .equip(EquipmentSlot::Weapon, "sword".to_string())
                .equip(EquipmentSlot::Armor, "plate".to_string())
                .equip(EquipmentSlot::Helmet, "helm".to_string())
                .equip(EquipmentSlot::Accessory, "ring".to_string());

            assert_eq!(equipment.weapon(), Some("sword"));
            assert_eq!(equipment.armor(), Some("plate"));
            assert_eq!(equipment.helmet(), Some("helm"));
            assert_eq!(equipment.accessory(), Some("ring"));
        }
    }

    // =========================================================================
    // ItemStack Tests
    // =========================================================================

    mod item_stack {
        use super::*;
        use crate::item::ItemIdentifier;

        #[rstest]
        fn new_creates_item_stack() {
            let item_id = ItemIdentifier::new();
            let stack = ItemStack::new(item_id, 5);
            assert_eq!(stack.item_identifier(), item_id);
            assert_eq!(stack.quantity(), 5);
        }

        #[rstest]
        fn new_with_zero_quantity() {
            let item_id = ItemIdentifier::new();
            let stack = ItemStack::new(item_id, 0);
            assert_eq!(stack.quantity(), 0);
        }

        #[rstest]
        fn add_quantity_increases_quantity() {
            let item_id = ItemIdentifier::new();
            let stack = ItemStack::new(item_id, 5);
            let increased = stack.add_quantity(3);
            assert_eq!(increased.quantity(), 8);
            assert_eq!(increased.item_identifier(), item_id);
        }

        #[rstest]
        fn add_quantity_saturates() {
            let item_id = ItemIdentifier::new();
            let stack = ItemStack::new(item_id, u32::MAX - 1);
            let increased = stack.add_quantity(10);
            assert_eq!(increased.quantity(), u32::MAX);
        }

        #[rstest]
        fn remove_quantity_decreases_quantity() {
            let item_id = ItemIdentifier::new();
            let stack = ItemStack::new(item_id, 5);
            let decreased = stack.remove_quantity(3).unwrap();
            assert_eq!(decreased.quantity(), 2);
        }

        #[rstest]
        fn remove_quantity_returns_none_when_equal() {
            let item_id = ItemIdentifier::new();
            let stack = ItemStack::new(item_id, 5);
            let result = stack.remove_quantity(5);
            assert!(result.is_none());
        }

        #[rstest]
        fn remove_quantity_returns_none_when_exceeds() {
            let item_id = ItemIdentifier::new();
            let stack = ItemStack::new(item_id, 5);
            let result = stack.remove_quantity(10);
            assert!(result.is_none());
        }

        #[rstest]
        fn display_format() {
            let item_id = ItemIdentifier::new();
            let stack = ItemStack::new(item_id, 5);
            let display = format!("{}", stack);
            assert!(display.contains("x5"));
        }

        #[rstest]
        fn equality() {
            let item_id = ItemIdentifier::new();
            let stack1 = ItemStack::new(item_id, 5);
            let stack2 = ItemStack::new(item_id, 5);
            let stack3 = ItemStack::new(item_id, 3);
            let stack4 = ItemStack::new(ItemIdentifier::new(), 5);

            assert_eq!(stack1, stack2);
            assert_ne!(stack1, stack3);
            assert_ne!(stack1, stack4);
        }

        #[rstest]
        fn clone() {
            let item_id = ItemIdentifier::new();
            let stack = ItemStack::new(item_id, 5);
            let cloned = stack.clone();
            assert_eq!(stack, cloned);
        }

        #[rstest]
        fn hash_consistency() {
            use std::collections::HashSet;

            let item_id = ItemIdentifier::new();
            let stack1 = ItemStack::new(item_id, 5);
            let stack2 = ItemStack::new(item_id, 5);
            let stack3 = ItemStack::new(item_id, 3);

            let mut set = HashSet::new();
            set.insert(stack1.clone());

            assert!(set.contains(&stack2));
            assert!(!set.contains(&stack3));
        }
    }

    // =========================================================================
    // Inventory Tests
    // =========================================================================

    mod inventory {
        use super::*;
        use crate::item::ItemIdentifier;

        #[rstest]
        fn new_creates_empty_inventory() {
            let inventory = Inventory::new(10);
            assert!(inventory.is_empty());
            assert_eq!(inventory.capacity(), 10);
            assert_eq!(inventory.len(), 0);
        }

        #[rstest]
        fn with_capacity_creates_empty_inventory() {
            let inventory = Inventory::with_capacity(15);
            assert!(inventory.is_empty());
            assert_eq!(inventory.capacity(), 15);
        }

        #[rstest]
        fn default_creates_inventory_with_default_capacity() {
            let inventory = Inventory::default();
            assert!(inventory.is_empty());
            assert_eq!(inventory.capacity(), 20);
        }

        #[rstest]
        fn is_full_when_at_capacity() {
            let item_id1 = ItemIdentifier::new();
            let item_id2 = ItemIdentifier::new();
            let inventory = Inventory::new(2)
                .add_item(item_id1, 1)
                .unwrap()
                .add_item(item_id2, 1)
                .unwrap();
            assert!(inventory.is_full());
        }

        #[rstest]
        fn is_full_when_not_at_capacity() {
            let item_id = ItemIdentifier::new();
            let inventory = Inventory::new(10).add_item(item_id, 1).unwrap();
            assert!(!inventory.is_full());
        }

        #[rstest]
        fn add_item_creates_new_stack() {
            let item_id = ItemIdentifier::new();
            let inventory = Inventory::new(10).add_item(item_id, 5).unwrap();
            assert_eq!(inventory.len(), 1);
            assert_eq!(inventory.quantity_of(&item_id), 5);
        }

        #[rstest]
        fn add_item_stacks_existing_items() {
            let item_id = ItemIdentifier::new();
            let inventory = Inventory::new(10)
                .add_item(item_id, 5)
                .unwrap()
                .add_item(item_id, 3)
                .unwrap();
            assert_eq!(inventory.len(), 1);
            assert_eq!(inventory.quantity_of(&item_id), 8);
        }

        #[rstest]
        fn add_item_different_items_create_separate_stacks() {
            let item_id1 = ItemIdentifier::new();
            let item_id2 = ItemIdentifier::new();
            let inventory = Inventory::new(10)
                .add_item(item_id1, 5)
                .unwrap()
                .add_item(item_id2, 3)
                .unwrap();
            assert_eq!(inventory.len(), 2);
            assert_eq!(inventory.quantity_of(&item_id1), 5);
            assert_eq!(inventory.quantity_of(&item_id2), 3);
        }

        #[rstest]
        fn add_item_fails_when_full() {
            let item_id1 = ItemIdentifier::new();
            let item_id2 = ItemIdentifier::new();
            let inventory = Inventory::new(1).add_item(item_id1, 1).unwrap();
            let result = inventory.add_item(item_id2, 1);
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                PlayerError::InventoryFull { capacity: 1 }
            ));
        }

        #[rstest]
        fn add_item_succeeds_when_full_but_stacking() {
            let item_id = ItemIdentifier::new();
            let inventory = Inventory::new(1).add_item(item_id, 1).unwrap();
            let result = inventory.add_item(item_id, 1);
            assert!(result.is_ok());
            assert_eq!(result.unwrap().quantity_of(&item_id), 2);
        }

        #[rstest]
        fn remove_item_decreases_quantity() {
            let item_id = ItemIdentifier::new();
            let inventory = Inventory::new(10).add_item(item_id, 5).unwrap();
            let updated = inventory.remove_item(&item_id, 3).unwrap();
            assert_eq!(updated.quantity_of(&item_id), 2);
        }

        #[rstest]
        fn remove_item_removes_stack_when_quantity_equals() {
            let item_id = ItemIdentifier::new();
            let inventory = Inventory::new(10).add_item(item_id, 5).unwrap();
            let updated = inventory.remove_item(&item_id, 5).unwrap();
            assert_eq!(updated.len(), 0);
            assert_eq!(updated.quantity_of(&item_id), 0);
        }

        #[rstest]
        fn remove_item_fails_when_quantity_exceeds() {
            let item_id = ItemIdentifier::new();
            let inventory = Inventory::new(10).add_item(item_id, 5).unwrap();
            let result = inventory.remove_item(&item_id, 10);
            assert!(result.is_err());
        }

        #[rstest]
        fn remove_item_fails_when_item_not_found() {
            let item_id = ItemIdentifier::new();
            let other_id = ItemIdentifier::new();
            let inventory = Inventory::new(10).add_item(item_id, 5).unwrap();
            let result = inventory.remove_item(&other_id, 1);
            assert!(result.is_err());
        }

        #[rstest]
        fn find_returns_item_stack() {
            let item_id = ItemIdentifier::new();
            let inventory = Inventory::new(10).add_item(item_id, 5).unwrap();
            let found = inventory.find(&item_id);
            assert!(found.is_some());
            assert_eq!(found.unwrap().quantity(), 5);
        }

        #[rstest]
        fn find_returns_none_when_not_found() {
            let item_id = ItemIdentifier::new();
            let inventory = Inventory::new(10);
            let found = inventory.find(&item_id);
            assert!(found.is_none());
        }

        #[rstest]
        fn items_returns_all_stacks() {
            let item_id1 = ItemIdentifier::new();
            let item_id2 = ItemIdentifier::new();
            let inventory = Inventory::new(10)
                .add_item(item_id1, 5)
                .unwrap()
                .add_item(item_id2, 3)
                .unwrap();
            let items = inventory.items();
            assert_eq!(items.len(), 2);
        }

        #[rstest]
        fn equality() {
            let item_id = ItemIdentifier::new();
            let inventory1 = Inventory::new(10).add_item(item_id, 5).unwrap();
            let inventory2 = Inventory::new(10).add_item(item_id, 5).unwrap();
            assert_eq!(inventory1, inventory2);
        }

        #[rstest]
        fn inequality_different_items() {
            let item_id1 = ItemIdentifier::new();
            let item_id2 = ItemIdentifier::new();
            let inventory1 = Inventory::new(10).add_item(item_id1, 5).unwrap();
            let inventory2 = Inventory::new(10).add_item(item_id2, 5).unwrap();
            assert_ne!(inventory1, inventory2);
        }

        #[rstest]
        fn inequality_different_capacity() {
            let inventory1 = Inventory::new(10);
            let inventory2 = Inventory::new(20);
            assert_ne!(inventory1, inventory2);
        }

        #[rstest]
        fn clone() {
            let item_id = ItemIdentifier::new();
            let inventory = Inventory::new(10).add_item(item_id, 5).unwrap();
            let cloned = inventory.clone();
            assert_eq!(inventory, cloned);
        }
    }
}
