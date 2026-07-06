//! Lecture de la config `influence-bot` par serveur (cf. coude/guild_settings).
//!
//! Fallback silencieux sur les defauts si le repo est indisponible ou la cle
//! absente. Le domaine reste pur : ces valeurs sont passees EN DONNEE aux
//! fonctions du domaine (seuils de paliers, argent de depart...).

use std::collections::HashMap;

use crate::domain::entities::influence::conversion::ConversionRates;
use crate::domain::entities::influence::tier::TierThresholds;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;

const BOT_NAME: &str = "influence-bot";

#[derive(Debug, Default, Clone)]
pub struct InfluenceSettings {
    raw: HashMap<String, String>,
}

impl InfluenceSettings {
    pub async fn load(repo: &dyn BotConfigRepository, guild_id: &str) -> Self {
        match repo.get_config(guild_id, BOT_NAME).await {
            Ok(entries) => Self {
                raw: entries
                    .into_iter()
                    .map(|e| (e.config_key, e.config_value))
                    .collect(),
            },
            Err(_) => Self::default(),
        }
    }

    fn get_i64(&self, key: &str, default: i64) -> i64 {
        self.raw
            .get(key)
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(default)
    }

    /// Argent de depart d'un nouveau citoyen.
    pub fn start_money(&self) -> i64 {
        self.get_i64("influence_start_money", 1000).max(0)
    }

    /// Cout de creation d'une organisation.
    pub fn org_creation_cost(&self) -> i64 {
        self.get_i64("influence_org_creation_cost", 1000).max(0)
    }

    /// Cout du role Discord d'une organisation (coins, gratuit pour un modo).
    pub fn org_role_cost(&self) -> i64 {
        self.get_i64("influence_org_role_cost", 2000).max(0)
    }

    /// Nombre max d'organisations qu'un citoyen peut fonder.
    pub fn org_max_per_citizen(&self) -> i64 {
        self.get_i64("influence_org_max_per_citizen", 3).max(0)
    }

    /// Duree du vote d'une loi (heures).
    pub fn law_debate_hours(&self) -> i64 {
        self.get_i64("influence_law_debate_hours", 48).max(1)
    }

    /// Cout d'une enquete (Argent).
    pub fn investigation_cost(&self) -> i64 {
        self.get_i64("influence_investigation_cost", 300).max(0)
    }

    /// Duree d'une enquete (heures).
    pub fn investigation_hours(&self) -> i64 {
        self.get_i64("influence_investigation_hours", 6).max(1)
    }

    /// Probabilite de reussite d'une enquete (0..=100).
    pub fn investigation_success_pct(&self) -> i64 {
        self.get_i64("influence_investigation_success_pct", 60).clamp(0, 100)
    }

    /// Reputation retiree a la cible d'un scandale.
    pub fn scandal_reputation_loss(&self) -> i64 {
        self.get_i64("influence_scandal_reputation_loss", 200).max(0)
    }

    /// Nombre d'evenements affiches par /actu et /archives.
    pub fn feed_size(&self) -> i64 {
        self.get_i64("influence_feed_size", 10).clamp(1, 25)
    }

    /// Seuils de paliers narratifs (defaut pour le MVP ; reglables plus tard).
    pub fn tier_thresholds(&self) -> TierThresholds {
        TierThresholds::default()
    }

    /// Taux de conversion des capitaux (cout en source par point de cible).
    pub fn conversion_rates(&self) -> ConversionRates {
        let d = ConversionRates::default();
        ConversionRates {
            money_to_reputation: self
                .get_i64("influence_conv_money_reputation", d.money_to_reputation),
            reputation_to_influence: self
                .get_i64("influence_conv_reputation_influence", d.reputation_to_influence),
            money_to_information: self
                .get_i64("influence_conv_money_information", d.money_to_information),
        }
    }
}
