//! Mode VOTE : alternative au mode review 1-clic (active par `vote_enabled`).
//!
//! Flux :
//!   1. Detection -> `post_vote_card` cree la review en API avec une echeance
//!      (`voting_deadline`, statut 'voting') et poste une carte avec des
//!      boutons de vote (Warn/Delete/Mute/Ban/Ignorer). custom_id
//!      `amv:<char>:<review_id>`.
//!   2. Chaque moderateur vote (`handle_vote_button`) -> POST /vote, la carte
//!      affiche le decompte a jour.
//!   3. A l'echeance, le worker appelle /decide -> event Redis
//!      `automod_review_decided` -> `handle_decided_event` edite la carte
//!      (verdict) et revele le bouton admin `amf:<review_id>`.
//!   4. L'admin clique (`handle_finalize_button`) -> POST /resolve
//!      (source=discord) + execution de la sanction Discord. L'admin
//!      confirme meme un refus (verdict 'ignore' = clore sans sanction).

mod buttons;
mod cards;
mod context;
mod discussion;
mod events;
mod finalize;
mod labels;
mod post;
mod render;

pub(super) const VOTE_PREFIX: &str = "amv:";
pub(super) const FINALIZE_PREFIX: &str = "amf:";
/// Bouton "Ouvrir une discussion" -> cree un salon textuel prive (membre + modos).
pub(super) const DISCUSSION_PREFIX: &str = "amdisc:";
/// Bouton "Clore (ignorer)" -> clot immediatement le dossier (tout moderateur).
pub(super) const CLOSE_PREFIX: &str = "amclose:";
/// Bouton moderateur : leve le timeout du membre concerne par la carte.
pub(super) const UNMUTE_PREFIX: &str = "amunmute:";
/// Bouton "Rouvrir le dossier" -> repasse en vote (tout moderateur).
pub(super) const REOPEN_PREFIX: &str = "amreopen:";

// Re-exports : conservent les chemins publics `vote::ITEM` attendus par les
// autres modules de l'automod (mod.rs, review.rs, backend.rs).
pub(crate) use buttons::{
    handle_close_button, handle_reopen_button, handle_unmute_button, handle_vote_button,
};
pub(crate) use cards::archive_discussion_channel;
pub(crate) use discussion::handle_discussion_button;
pub(crate) use events::{handle_card_expired_event, handle_decided_event};
pub(crate) use finalize::{
    apply_member_sanction, handle_finalize_button, log_sanction_to_moderation,
};
pub(crate) use post::{post_manual_vote_card, post_vote_card};
pub(crate) use render::{build_detail_url, render_history_totals};
