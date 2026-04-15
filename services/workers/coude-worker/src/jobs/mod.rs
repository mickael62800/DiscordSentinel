pub mod expire_combats;
pub mod hp_regen;
pub mod resolve_betting;
// Phase 3 partiel : wallet_log est encore utilise par expire_combats (qui
// sera migre vers un RPC API dans une future phase). A supprimer une fois
// expire_combats thin.
pub mod wallet_log;
