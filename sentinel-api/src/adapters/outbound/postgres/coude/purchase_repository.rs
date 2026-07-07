//! Impl Postgres de `PurchaseRepository` : achat boutique atomique.
//!
//! Une seule transaction couvre tout le flux economique d'un achat :
//! 1. verrou du wallet (`SELECT ... FOR UPDATE`) + lecture du solde ;
//! 2. si solde < prix : rollback, retour `InsufficientFunds` (aucune mutation) ;
//! 3. debit du wallet partage (`user_wallets`) + ligne de ledger ;
//! 4. ajout de l'item a l'inventaire (`coude_inventory`) ;
//! 5. alimentation de la caisse communautaire (`coude_cashbox`).
//!
//! L'atomicite est garantie par la transaction : si une etape echoue, TOUT est
//! rollback nativement — plus de compensation manuelle cote bot, donc plus de
//! risque de perte de coins.

use async_trait::async_trait;
use sqlx::PgPool;

use crate::adapters::outbound::postgres::casino::wallet_tx_log::log_wallet_tx;
use crate::ports::outbound::coude::purchase_repository::PurchaseRepository;
use crate::ports::outbound::coude::purchase_repository::PurchaseTxOutcome;
use sentinel_core::domain::errors::DomainError;

use super::super::pg_err;

pub struct PgPurchaseRepository {
    pool: PgPool,
}

impl PgPurchaseRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PurchaseRepository for PgPurchaseRepository {
    async fn purchase_item_atomic(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
        price: i64,
    ) -> Result<PurchaseTxOutcome, DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;

        // 1. Verrou du wallet partage + lecture du solde (absence => 0).
        let balance: i64 = sqlx::query_scalar(
            "SELECT coins FROM user_wallets WHERE guild_id = $1 AND user_id = $2 FOR UPDATE",
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(pg_err)?
        .unwrap_or(0);

        // 2. Solde insuffisant : on annule (rien n'a ete modifie) et on signale.
        if balance < price {
            tx.rollback().await.map_err(pg_err)?;
            return Ok(PurchaseTxOutcome::InsufficientFunds { balance });
        }

        // 3. Debit atomique + ledger consultable via /resume.
        let new_balance: i64 = sqlx::query_scalar(
            "UPDATE user_wallets \
                SET coins = coins - $3, total_spent = total_spent + $3, updated_at = NOW() \
             WHERE guild_id = $1 AND user_id = $2 \
             RETURNING coins",
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(price)
        .fetch_one(&mut *tx)
        .await
        .map_err(pg_err)?;

        log_wallet_tx(
            &mut tx,
            guild_id,
            user_id,
            -price,
            new_balance,
            "coude_shop_purchase",
            &format!("Achat boutique : {item_key}"),
        )
        .await?;

        // 4. Ajout de l'item a l'inventaire.
        sqlx::query(
            r#"INSERT INTO coude_inventory (guild_id, user_id, item_key, quantity)
               VALUES ($1, $2, $3, 1)
               ON CONFLICT (guild_id, user_id, item_key)
               DO UPDATE SET quantity = coude_inventory.quantity + 1"#,
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(item_key)
        .execute(&mut *tx)
        .await
        .map_err(pg_err)?;

        // 5. Alimentation de la caisse communautaire (meme transaction).
        sqlx::query(
            r#"INSERT INTO coude_cashbox (guild_id, balance, total_collected)
               VALUES ($1, $2, $2)
               ON CONFLICT (guild_id) DO UPDATE SET
                   balance = coude_cashbox.balance + EXCLUDED.balance,
                   total_collected = coude_cashbox.total_collected + EXCLUDED.total_collected,
                   updated_at = NOW()"#,
        )
        .bind(guild_id)
        .bind(price)
        .execute(&mut *tx)
        .await
        .map_err(pg_err)?;

        tx.commit().await.map_err(pg_err)?;
        Ok(PurchaseTxOutcome::Purchased { new_balance })
    }
}
