//! Helpers partages pour le rendu SVG -> PNG via resvg (classements, ...).
//! Evite de dupliquer le chargement des fonts systeme,
//! l'echappement XML et l'encodage base64 dans chaque module de rendu.

use std::sync::Arc;

use base64::Engine;
use once_cell::sync::OnceCell;
use resvg::usvg;

static FONTDB: OnceCell<Arc<usvg::fontdb::Database>> = OnceCell::new();

/// Base de fonts systeme partagee (chargee une seule fois pour tout le process).
pub fn fontdb() -> Arc<usvg::fontdb::Database> {
    FONTDB
        .get_or_init(|| {
            let mut db = usvg::fontdb::Database::new();
            db.load_system_fonts();
            Arc::new(db)
        })
        .clone()
}

/// Echappe les caracteres speciaux XML/HTML pour insertion dans du SVG.
pub fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Encode des octets en base64 standard (pour les data URIs SVG).
pub fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}
