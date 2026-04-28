use serde::Deserialize;
use serde::Serialize;
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlagType {
    Spam,
    Insult,
    Link,
    Phishing,
    // IA Vision
    Nsfw,
    Illicit,
    // IA Text Sentiment
    Anger,
    Rage,
    Threat,
    Harassment,
}

impl FlagType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FlagType::Spam => "spam",
            FlagType::Insult => "insult",
            FlagType::Link => "link",
            FlagType::Phishing => "phishing",
            FlagType::Nsfw => "nsfw",
            FlagType::Illicit => "illicit",
            FlagType::Anger => "anger",
            FlagType::Rage => "rage",
            FlagType::Threat => "threat",
            FlagType::Harassment => "harassment",
        }
    }

    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "spam" => FlagType::Spam,
            "insult" => FlagType::Insult,
            "link" => FlagType::Link,
            "phishing" => FlagType::Phishing,
            "nsfw" => FlagType::Nsfw,
            "illicit" => FlagType::Illicit,
            "anger" => FlagType::Anger,
            "rage" => FlagType::Rage,
            "threat" => FlagType::Threat,
            "harassment" => FlagType::Harassment,
            _ => FlagType::Spam,
        }
    }
}

#[cfg(test)]
#[path = "tests/flag_type.rs"]
mod tests;
