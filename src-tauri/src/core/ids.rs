use serde::{Deserialize, Serialize};
use std::fmt;

macro_rules! define_id {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(pub String);

        impl $name {
            pub fn new() -> Self {
                Self(uuid::Uuid::new_v4().to_string())
            }

            pub fn from_string(s: String) -> Self {
                Self(s)
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                Self(s)
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                Self(s.to_string())
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }
    };
}

define_id!(BookId);
define_id!(ChapterId);
define_id!(CardId);
define_id!(EntityId);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_book_id_new() {
        let id1 = BookId::new();
        let id2 = BookId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_id_from_string() {
        let id = BookId::from_string("test-id".to_string());
        assert_eq!(id.as_str(), "test-id");
    }

    #[test]
    fn test_id_display() {
        let id = ChapterId::from_string("chapter-1".to_string());
        assert_eq!(format!("{}", id), "chapter-1");
    }

    #[test]
    fn test_id_serialization() {
        let id = CardId::from_string("card-123".to_string());
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"card-123\"");

        let deserialized: CardId = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, id);
    }
}
