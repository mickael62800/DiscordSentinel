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
mod tests {
    use super::*;

    #[test]
    fn test_no_active_flags() {
        let flags = DetectionFlags { spam: false, insult: false, link: false, phishing: false };
        assert!(flags.active_flags().is_empty());
    }

    #[test]
    fn test_all_active_flags() {
        let flags = DetectionFlags { spam: true, insult: true, link: true, phishing: true };
        let active = flags.active_flags();
        assert_eq!(active.len(), 4);
    }

    #[test]
    fn test_single_flag_spam() {
        let flags = DetectionFlags { spam: true, insult: false, link: false, phishing: false };
        let active = flags.active_flags();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0], FlagType::Spam);
    }

    #[test]
    fn test_phishing_default_false_serde() {
        let json = r#"{"spam": true, "insult": false, "link": false}"#;
        let flags: DetectionFlags = serde_json::from_str(json).unwrap();
        assert!(flags.spam);
        assert!(!flags.phishing); // default
    }
}
