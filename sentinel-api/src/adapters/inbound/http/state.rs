//! Re-export de `AppState` pour les chemins historiques.
//!
//! Le type vit desormais dans `crate::bootstrap::state` : c'est la composition
//! root, pas un detail de l'adaptateur HTTP. Le laisser ici obligeait
//! l'adaptateur gRPC a faire
//! `use crate::adapters::inbound::http::state::AppState`, c'est-a-dire a
//! dependre d'un adaptateur frere.
//!
//! Ce module ne survit que pour eviter de reecrire ~380 imports d'un coup.
//! Les nouveaux fichiers doivent importer `crate::bootstrap::AppState`.

pub use crate::bootstrap::state::AppState;
