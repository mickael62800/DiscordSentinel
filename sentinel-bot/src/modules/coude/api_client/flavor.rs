//! API client catalogue de templates flavor (Phase 3 #9 audit).
//!
//! Le bot tire un template au hasard pour `(key, locale)`. Si l'API
//! repond 404 (aucun template) ou Err, le caller fallback sur ses
//! arrays locales pour preserver le comportement legacy — important :
//! la migration ne doit pas casser /voler /braquage /prank si la table
//! n'a pas encore ete seedee ou si l'API est down.

use serde::Deserialize;

use super::ApiClient;

#[derive(Debug, Deserialize, Clone)]
pub struct FlavorTemplateResp {
    pub content: String,
}

impl ApiClient {
    /// Tire un template aleatoire depuis l'API. `Ok(None)` si la cle est
    /// inconnue (404), `Err` sur autre erreur reseau/serveur.
    pub async fn random_flavor(&self, key: &str, locale: &str) -> Result<Option<String>, String> {
        // Les keys sont des identifiants ASCII (steal_success_afk, etc.) et
        // locale est "fr"/"en" — pas besoin de url-encoder.
        let path = format!("/api/coude/flavor/{}/random?locale={}", key, locale);
        match self.base.get_json::<FlavorTemplateResp>(&path).await {
            Ok(r) => Ok(Some(r.content)),
            Err(e) if e.contains("404") || e.to_lowercase().contains("not found") => Ok(None),
            Err(e) => Err(e),
        }
    }
}
