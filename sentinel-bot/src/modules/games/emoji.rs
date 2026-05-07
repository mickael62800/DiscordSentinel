//! Helpers emoji : parsing d'une chaine stockee en DB et comparaison avec
//! un `ReactionType` recu de Discord.

use serenity::all::{EmojiId, ReactionType};

/// Parse une chaine emoji :
/// - `<:name:123456>` → custom
/// - `<a:name:123456>` → custom anime
/// - sinon → unicode (ex. "🎮")
///
/// Retourne None si la chaine est vide.
pub fn parse_reaction_type(raw: &str) -> Option<ReactionType> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }

    if let Some(custom) = parse_custom(s) {
        return Some(custom);
    }

    // Fallback : unicode. On ne tente pas de valider les codepoints,
    // Discord rejettera au besoin.
    Some(ReactionType::Unicode(s.to_string()))
}

fn parse_custom(s: &str) -> Option<ReactionType> {
    // Formats : <:name:id> ou <a:name:id>
    let inner = s.strip_prefix('<')?.strip_suffix('>')?;
    let (animated, rest) = if let Some(r) = inner.strip_prefix("a:") {
        (true, r)
    } else if let Some(r) = inner.strip_prefix(':') {
        (false, r)
    } else {
        return None;
    };
    let (name, id_str) = rest.rsplit_once(':')?;
    let id: u64 = id_str.parse().ok()?;
    Some(ReactionType::Custom {
        animated,
        id: EmojiId::new(id),
        name: Some(name.to_string()),
    })
}

/// Compare un emoji stocke en DB avec un ReactionType recu de Discord.
#[allow(dead_code)]
pub fn emoji_matches(stored: &str, reaction: &ReactionType) -> bool {
    let parsed = match parse_reaction_type(stored) {
        Some(p) => p,
        None => return false,
    };
    match (&parsed, reaction) {
        (ReactionType::Unicode(a), ReactionType::Unicode(b)) => a == b,
        (
            ReactionType::Custom { id: a, .. },
            ReactionType::Custom { id: b, .. },
        ) => a == b,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_unicode() {
        let r = parse_reaction_type("🎮").unwrap();
        assert!(matches!(r, ReactionType::Unicode(_)));
    }

    #[test]
    fn parse_custom_static() {
        let r = parse_reaction_type("<:cool:123456789>").unwrap();
        match r {
            ReactionType::Custom { animated, id, name } => {
                assert!(!animated);
                assert_eq!(id.get(), 123456789);
                assert_eq!(name.as_deref(), Some("cool"));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn parse_custom_animated() {
        let r = parse_reaction_type("<a:wave:987654321>").unwrap();
        match r {
            ReactionType::Custom { animated, id, .. } => {
                assert!(animated);
                assert_eq!(id.get(), 987654321);
            }
            _ => panic!(),
        }
    }

    #[test]
    fn matches_unicode() {
        let r = ReactionType::Unicode("🎮".to_string());
        assert!(emoji_matches("🎮", &r));
        assert!(!emoji_matches("🎯", &r));
    }

    #[test]
    fn matches_custom_by_id() {
        let r = ReactionType::Custom {
            animated: false,
            id: EmojiId::new(42),
            name: Some("x".into()),
        };
        assert!(emoji_matches("<:other:42>", &r));
        assert!(!emoji_matches("<:other:99>", &r));
    }
}
