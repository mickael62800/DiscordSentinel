use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlagType {
    Spam,
    Insult,
    Link,
}

impl FlagType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FlagType::Spam => "spam",
            FlagType::Insult => "insult",
            FlagType::Link => "link",
        }
    }

    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "spam" => FlagType::Spam,
            "insult" => FlagType::Insult,
            "link" => FlagType::Link,
            _ => FlagType::Spam,
        }
    }
}
