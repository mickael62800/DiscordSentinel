//! Impl de la caisse communautaire Coude (Phase 9).
//!
//! Gere les deposits (flux qui retirent des coins de l'economie) et la
//! redistribution hebdomadaire aleatoire aux joueurs actifs.
//!
//! ## Algo de distribution aleatoire
//!
//! But : avoir un effet loterie avec des gains disparates. Quelqu'un peut
//! toucher 50 % de la caisse, un autre 1 %, un autre 0.5 %, etc. On genere
//! N poids aleatoires (N = nombre de joueurs actifs, cape a 20) puis on
//! normalise. Les joueurs hors du top 20 ne touchent rien (loterie = peu
//! de grosses mains).
//!
//! Les poids sont triés par ordre décroissant pour que la sortie montre
//! visuellement le classement des gains (du plus gros au plus petit), mais
//! la selection des gagnants est totalement aleatoire.

use std::sync::Arc;

use async_trait::async_trait;
use rand::seq::SliceRandom;
use rand::Rng;
use tracing::info;
use tracing::warn;
use uuid::Uuid;

use crate::domain::entities::coude::cashbox::Cashbox;
use crate::domain::entities::coude::cashbox::CashboxRedistribution;
use crate::domain::entities::coude::cashbox::CashboxRedistributionEntry;
use crate::domain::entities::coude::cashbox::CashboxSource;
use crate::domain::errors::DomainError;
use crate::ports::inbound::coude::manage_cashbox::ManageCoudeCashboxUseCase;
use crate::ports::inbound::coude::manage_cashbox::RedistributionOutcome;
use crate::ports::outbound::casino::wallet_repository::WalletRepository;
use crate::ports::outbound::coude::cashbox_repository::CashboxRepository;
/// Nombre max de gagnants par redistribution. Au-dela, on ne cape pas
/// strictement : on met la valeur en env var lors de l'init ou on cape ici.
///
/// **Choix d'architecture** : hardcode. 20 gagnants maximum garantit
/// des gains individuels visibles meme pour une caisse modeste (>= 1
/// coin par gagnant a partir de 20c dans la caisse). Le rendre
/// configurable apporte peu ; pour tuner, modifier ici et redeployer.
const MAX_WINNERS: usize = 20;
/// Fenetre de joueurs "actifs" (jours). Hardcode pour les memes raisons
/// que `MAX_WINNERS` — 7 jours couvre un cycle hebdo de redistribution
/// et filtre correctement les lurkers sans exclure les joueurs reguliers.
const ACTIVE_WINDOW_DAYS: i64 = 7;

pub struct ManageCoudeCashboxService {
    repo: Arc<dyn CashboxRepository>,
    wallet_repo: Arc<dyn WalletRepository>,
}

impl ManageCoudeCashboxService {
    pub fn new(repo: Arc<dyn CashboxRepository>, wallet_repo: Arc<dyn WalletRepository>) -> Self {
        Self { repo, wallet_repo }
    }

    /// Genere N poids aleatoires exponentiels puis les normalise pour
    /// sommer a `total`. Les poids sont tries par ordre decroissant.
    /// Retourne une liste de montants entiers (i64).
    fn distribute_random(total: i64, n: usize) -> Vec<i64> {
        if n == 0 || total <= 0 {
            return vec![];
        }
        let mut rng = rand::thread_rng();
        // Distribution exponentielle : de gros ecarts entre 1er et dernier.
        let mut raw: Vec<f64> = (0..n)
            .map(|_| {
                let r: f64 = rng.gen_range(0.001..1.0);
                // -ln(r) donne une distribution exponentielle (memoryless).
                -r.ln()
            })
            .collect();
        let sum: f64 = raw.iter().sum();
        if sum == 0.0 {
            return vec![];
        }
        // Normalise + convertit en i64 avec cumul compense pour eviter
        // les erreurs d'arrondi (le dernier prend le reliquat).
        let mut amounts: Vec<i64> = raw
            .iter()
            .map(|w| ((w / sum) * total as f64).floor() as i64)
            .collect();
        let distributed: i64 = amounts.iter().sum();
        let remainder = total - distributed;
        if remainder > 0 && !amounts.is_empty() {
            amounts[0] += remainder; // donne le reliquat au plus gros gagnant
        }
        amounts.sort_unstable_by(|a, b| b.cmp(a)); // tri decroissant
        raw.clear();
        amounts
    }
}

#[async_trait]
impl ManageCoudeCashboxUseCase for ManageCoudeCashboxService {
    async fn get_cashbox(&self, guild_id: &str) -> Result<Cashbox, DomainError> {
        self.repo.get_or_create(guild_id).await
    }

    async fn deposit(
        &self,
        guild_id: &str,
        amount: i64,
        source: CashboxSource,
    ) -> Result<(), DomainError> {
        if amount <= 0 {
            return Ok(()); // no-op silencieux
        }
        self.repo.deposit(guild_id, amount, source).await
    }

    async fn redistribute_weekly(
        &self,
        guild_id: &str,
    ) -> Result<Option<RedistributionOutcome>, DomainError> {
        // 1. Liste des joueurs actifs (7j)
        let active = self
            .repo
            .list_active_players(guild_id, ACTIVE_WINDOW_DAYS)
            .await?;
        if active.is_empty() {
            info!(
                guild_id,
                "Redistribution skip : aucun joueur actif dans les 7j"
            );
            return Ok(None);
        }

        // 2. Claim atomique du contenu de la caisse
        let total = self.repo.claim_all_for_redistribution(guild_id).await?;
        if total <= 0 {
            info!(guild_id, "Redistribution skip : caisse vide");
            return Ok(None);
        }

        // 3. Sous-echantillonne MAX_WINNERS + shuffle + compute amounts.
        //    Tout ca dans un bloc scope pour que ThreadRng (pas Send) soit
        //    drop avant tous les await qui suivent.
        let (players, amounts) = {
            let mut rng = rand::thread_rng();
            let mut players = active;
            if players.len() > MAX_WINNERS {
                players.shuffle(&mut rng);
                players.truncate(MAX_WINNERS);
            }
            let amounts = Self::distribute_random(total, players.len());
            players.shuffle(&mut rng);
            (players, amounts)
        };

        if amounts.is_empty() {
            warn!(guild_id, total, "distribute_random a renvoye vide, abort");
            // On remet le total dans la caisse pour ne pas perdre les coins
            let _ = self
                .repo
                .deposit(guild_id, total, CashboxSource::ShopPurchase)
                .await;
            return Ok(None);
        }

        // 4. Match joueurs -> amounts (vecs deja melanges)
        let winners: Vec<(String, String, i64)> = players
            .into_iter()
            .zip(amounts.iter().copied())
            .filter(|(_, amt)| *amt > 0)
            .map(|((uid, uname), amt)| (uid, uname, amt))
            .collect();

        // 6. Credit chaque gagnant sur son wallet + persiste l'historique
        let redistribution_id = self
            .repo
            .record_redistribution(guild_id, total, winners.clone())
            .await?;

        // Fix #7 (anti-destruction) : la caisse a deja ete videe atomiquement
        // (claim_all_for_redistribution). Si un credit gagnant echoue, ses coins
        // seraient perdus a jamais. On accumule les montants non distribues et
        // on les re-depose dans la caisse en fin de boucle (compensation) afin
        // qu'ils restent dans l'economie pour le prochain cycle — aucune
        // destruction nette.
        let mut undistributed: i64 = 0;
        for (user_id, _username, amount_won) in &winners {
            let desc = format!("Redistribution hebdomadaire caisse coude #{redistribution_id}");
            if let Err(e) = self
                .wallet_repo
                .credit(
                    guild_id,
                    user_id,
                    *amount_won,
                    "coude_cashbox_redist",
                    &desc,
                )
                .await
            {
                warn!(error = %e, user_id, amount_won, "Echec credit redistribution : montant re-depose dans la caisse");
                undistributed += *amount_won;
            }
        }

        // Re-banque le reliquat non distribue pour ne pas detruire de coins.
        if undistributed > 0 {
            if let Err(e) = self
                .repo
                .deposit(guild_id, undistributed, CashboxSource::ShopPurchase)
                .await
            {
                warn!(
                    error = %e, guild_id, undistributed,
                    "Echec re-depot du reliquat non distribue : coins potentiellement perdus"
                );
            }
        }

        info!(
            guild_id,
            total,
            winners = winners.len(),
            redistribution_id = %redistribution_id,
            "Cashbox redistributee"
        );

        Ok(Some(RedistributionOutcome {
            redistribution_id,
            total_amount: total,
            winners,
        }))
    }

    async fn list_redistributions(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<CashboxRedistribution>, DomainError> {
        self.repo.list_redistributions(guild_id, limit).await
    }

    async fn list_entries(
        &self,
        redistribution_id: Uuid,
    ) -> Result<Vec<CashboxRedistributionEntry>, DomainError> {
        self.repo.list_entries(redistribution_id).await
    }

    async fn redistribute_due_guilds(
        &self,
        min_days_since_last: i64,
    ) -> Result<Vec<(String, RedistributionOutcome)>, DomainError> {
        let guilds = self
            .repo
            .list_guilds_due_for_redistribution(min_days_since_last)
            .await?;
        let mut out = Vec::with_capacity(guilds.len());
        for guild_id in guilds {
            match self.redistribute_weekly(&guild_id).await {
                Ok(Some(outcome)) => {
                    info!(
                        guild_id,
                        winners = outcome.winners.len(),
                        "Cashbox redistributee via worker hebdo"
                    );
                    out.push((guild_id, outcome));
                }
                Ok(None) => {
                    // Caisse claim a retourne 0 ou aucun joueur actif —
                    // transition benigne, on passe a la guild suivante.
                }
                Err(e) => {
                    warn!(error = %e, guild_id, "Echec redistribution guild, on passe a la suivante");
                }
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
#[path = "tests/manage_cashbox.rs"]
mod tests;
