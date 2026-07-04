//! Jeu « Influence » — entites de domaine (cf. docs/Nouveau jeux/ARCHITECTURE.md).
//!
//! Phase 1 (MVP) : citoyen + 5 capitaux + paliers narratifs. Les organisations,
//! votes, lois, etc. s'ajoutent aux phases suivantes.

pub mod capital;
pub mod citizen;
pub mod conversion;
pub mod law;
pub mod motion;
pub mod movement;
pub mod org_membership;
pub mod organization;
pub mod tier;
pub mod vote;
