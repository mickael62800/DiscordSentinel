//! Implementation du duel amical (cf. COUPE_AMELIORATIONS 4.5).
//!
//! On reutilise le moteur de combat pur (`coude_combat_engine`) avec
//! mise=0, sans event ni curse, avec les params par defaut. Le resultat
//! cote economie (coins_won, coins_lost_by_loser) est ignore — c est le
//! point precis de "duel amical sans risque".

use std::sync::Arc;

use async_trait::async_trait;

use crate::application::coude::guild_settings::GuildSettings;
use crate::domain::entities::coude::balance::BalanceParams;
use crate::domain::errors::DomainError;
use crate::domain::services::coude::coude_combat_engine::combat::resolve_combat;
use crate::domain::services::coude::coude_combat_engine::PlayerLite;
use crate::ports::inbound::coude::manage_players::ManageCoudePlayersUseCase;
use crate::ports::inbound::coude::resolve_friendly_duel::FriendlyDuelInput;
use crate::ports::inbound::coude::resolve_friendly_duel::FriendlyDuelOutput;
use crate::ports::inbound::coude::resolve_friendly_duel::ResolveFriendlyDuelUseCase;
use crate::ports::outbound::coude::player_repository::PlayerRepository;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;
const DEFAULT_FRIENDLY_WINNER_XP: i64 = 20;
const DEFAULT_FRIENDLY_LOSER_XP: i64 = 5;

pub struct ResolveFriendlyDuelService {
    pub player_repo: Arc<dyn PlayerRepository>,
    pub players_uc: Arc<dyn ManageCoudePlayersUseCase>,
    pub bot_config_repo: Arc<dyn BotConfigRepository>,
}

impl ResolveFriendlyDuelService {
    pub fn new(
        player_repo: Arc<dyn PlayerRepository>,
        players_uc: Arc<dyn ManageCoudePlayersUseCase>,
        bot_config_repo: Arc<dyn BotConfigRepository>,
    ) -> Self {
        Self {
            player_repo,
            players_uc,
            bot_config_repo,
        }
    }
}

#[async_trait]
impl ResolveFriendlyDuelUseCase for ResolveFriendlyDuelService {
    async fn resolve(&self, input: FriendlyDuelInput) -> Result<FriendlyDuelOutput, DomainError> {
        if input.attacker_id == input.defender_id {
            return Err(DomainError::ValidationError(
                "Tu ne peux pas te defier toi-meme.".into(),
            ));
        }

        // Les deux joueurs sont distincts (verifie ci-dessus) → fetch en parallele.
        let (attacker, defender) = tokio::try_join!(
            self.player_repo.get_or_create(
                &input.guild_id,
                &input.attacker_id,
                &input.attacker_name
            ),
            self.player_repo.get_or_create(
                &input.guild_id,
                &input.defender_id,
                &input.defender_name
            ),
        )?;

        let attacker_lite = PlayerLite {
            user_id: attacker.user_id.clone(),
            class: attacker.class.as_ref().map(|c| c.as_str().to_string()),
            level: attacker.level,
            atk: attacker.atk,
            def: attacker.def,
            cowardice_count: attacker.cowardice_count,
            hp_current: Some(attacker.hp_current),
        };
        let defender_lite = PlayerLite {
            user_id: defender.user_id.clone(),
            class: defender.class.as_ref().map(|c| c.as_str().to_string()),
            level: defender.level,
            atk: defender.atk,
            def: defender.def,
            cowardice_count: defender.cowardice_count,
            hp_current: Some(defender.hp_current),
        };

        let params = BalanceParams::default();
        let result = resolve_combat(
            &attacker_lite,
            &defender_lite,
            attacker.hp_current,
            defender.hp_current,
            0, // mise nulle : moteur tourne sans payout
            None,
            None,
            &[],
            &params,
        );

        let settings = GuildSettings::load(&*self.bot_config_repo, &input.guild_id).await;
        let cfg_winner_xp = settings.get_i64("friendly_winner_xp", DEFAULT_FRIENDLY_WINNER_XP);
        let cfg_loser_xp = settings.get_i64("friendly_loser_xp", DEFAULT_FRIENDLY_LOSER_XP);

        let draw = result.winner_id.is_none() && result.loser_id.is_none();
        let (winner_xp, loser_xp) = if draw {
            (cfg_loser_xp, cfg_loser_xp)
        } else {
            (cfg_winner_xp, cfg_loser_xp)
        };

        if let Some(winner_id) = &result.winner_id {
            let _ = self
                .player_repo
                .increment_friendly_stat(&input.guild_id, winner_id, true)
                .await;
            let _ = self
                .players_uc
                .add_xp(&input.guild_id, winner_id, winner_xp)
                .await;
        }
        if let Some(loser_id) = &result.loser_id {
            let _ = self
                .player_repo
                .increment_friendly_stat(&input.guild_id, loser_id, false)
                .await;
            let _ = self
                .players_uc
                .add_xp(&input.guild_id, loser_id, loser_xp)
                .await;
        }

        Ok(FriendlyDuelOutput {
            winner_id: result.winner_id,
            loser_id: result.loser_id,
            draw,
            total_rounds: result.total_rounds,
            attacker_hp_final: result.attacker_hp_final,
            attacker_hp_max: result.attacker_hp_max,
            defender_hp_final: result.defender_hp_final,
            defender_hp_max: result.defender_hp_max,
            winner_xp,
            loser_xp,
        })
    }
}
