use serde::{Deserialize, Serialize};

use super::FlagType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionFlags {
    pub spam: bool,
    pub insult: bool,
    pub link: bool,
    #[serde(default)]
    pub phishing: bool,
}

impl DetectionFlags {
    pub fn active_flags(&self) -> Vec<FlagType> {
        let mut flags = Vec::new();
        if self.spam {
            flags.push(FlagType::Spam);
        }
        if self.insult {
            flags.push(FlagType::Insult);
        }
        if self.link {
            flags.push(FlagType::Link);
        }
        if self.phishing {
            flags.push(FlagType::Phishing);
        }
        flags
    }
}

#[cfg(test)]
#[path = "tests/detection_flags.rs"]
mod tests;
