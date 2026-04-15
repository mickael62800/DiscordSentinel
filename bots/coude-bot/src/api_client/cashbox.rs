//! Methodes `ApiClient` de la caisse communautaire (`/cagnotte`).
//!
//! Lecture de l'etat (balance, total_collected, etc.) et depot depuis
//! les flux du jeu qui retirent des coins (shop, assurance, taxe,
//! penalite lachete). Le depot est best-effort : un echec est logge
//! mais ne bloque pas l'appelant.

use sentinel_proto::coude::v1 as proto_coude;

use super::{grpc_err_to_string, ApiClient, Cashbox, CashboxDepositSource};

impl ApiClient {
    /// Lit l'etat courant de la caisse communautaire d'une guild.
    pub async fn get_cashbox(&self, guild_id: &str) -> Result<Cashbox, String> {
        let req = proto_coude::GetCashboxRequest {
            guild_id: guild_id.to_string(),
        };
        let mut client = self.grpc.coude_social();
        let r = self
            .grpc
            .guarded(|| async move { client.get_cashbox(req).await.map(|r| r.into_inner()) })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(Cashbox {
            guild_id: r.guild_id,
            balance: r.balance,
            total_collected: r.total_collected,
            total_redistributed: r.total_redistributed,
            last_redistribution_at: r.last_redistribution_at,
        })
    }

    /// Depose un montant dans la caisse communautaire. Best-effort :
    /// une erreur est journalisee mais ne bloque pas l'appelant, pour
    /// que l'achat principal n'echoue pas si la caisse est indisponible.
    pub async fn deposit_cashbox(
        &self,
        guild_id: &str,
        amount: i64,
        source: CashboxDepositSource,
    ) -> Result<(), String> {
        if amount <= 0 {
            return Ok(());
        }
        let req = proto_coude::DepositCashboxRequest {
            guild_id: guild_id.to_string(),
            amount,
            source: source.as_proto() as i32,
        };
        let mut client = self.grpc.coude_social();
        self.grpc
            .guarded(|| async move { client.deposit_cashbox(req).await.map(|_| ()) })
            .await
            .map_err(grpc_err_to_string)
    }
}
