#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FormulaSpaceId(String);

impl FormulaSpaceId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, HashMap};

    #[test]
    fn formula_space_id_round_trips_string_value() {
        let id = FormulaSpaceId::new("space-1");
        assert_eq!(id.as_str(), "space-1");
        assert_eq!(
            FormulaSpaceId::new(String::from("space-2")).as_str(),
            "space-2"
        );
    }

    #[test]
    fn formula_space_id_equality_and_ordering_are_consistent() {
        let a = FormulaSpaceId::new("alpha");
        let b = FormulaSpaceId::new("bravo");
        let a_again = FormulaSpaceId::new("alpha");

        // Equality is by wrapped string value.
        assert_eq!(a, a_again);
        assert_ne!(a, b);
        // Ordering matches the wrapped string ordering so BTreeMap lookups
        // behave predictably for the state's formula space map.
        assert!(a < b);

        let mut btree = BTreeMap::new();
        btree.insert(a.clone(), "first");
        btree.insert(b.clone(), "second");
        assert_eq!(btree.get(&a_again).copied(), Some("first"));

        // Hash equality matches value equality — two ids built from the
        // same string must collide in a HashMap.
        let mut hashed: HashMap<FormulaSpaceId, usize> = HashMap::new();
        hashed.insert(a.clone(), 1);
        assert_eq!(hashed.get(&a_again).copied(), Some(1));
    }
}
