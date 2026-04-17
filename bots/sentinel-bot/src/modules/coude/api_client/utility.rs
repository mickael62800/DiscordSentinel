//! Methodes utilitaires diverses du `ApiClient`, sans lien fort avec
//! un domaine metier : enumerations admin (guilds enregistrees),
//! historique wallet, tirage aleatoire de joueurs.
//!
//! Ces trois methodes n'ont PAS d'equivalent gRPC et passent encore
//! par HTTP legacy (`base.get_json`). Utilisees par des commandes
//! rares (admin/debug).

use super::{ApiClient, Player, WalletTransaction};

impl ApiClient {
    pub async fn get_all_guild_ids(&self) -> Result<Vec<String>, String> {
        self.base.get_json("/api/coude/guilds").await
    }

    pub async fn get_wallet_transactions(
        &self,
        guild_id: &str,
        user_id: &str,
        limit: i64,
    ) -> Result<Vec<WalletTransaction>, String> {
        self.base
            .get_json(&format!(
                "/api/wallet/{guild_id}/{user_id}/transactions?limit={limit}"
            ))
            .await
    }

    pub async fn get_random_players(
        &self,
        guild_id: &str,
        count: i64,
    ) -> Result<Vec<Player>, String> {
        self.base
            .get_json(&format!(
                "/api/coude/{guild_id}/players/random?count={count}"
            ))
            .await
    }
}
