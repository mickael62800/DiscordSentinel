pub mod accepter;
pub mod aide;
pub mod annuler;
pub mod cagnotte;
pub mod classe;
pub mod coude;
pub mod coude_amical;
pub mod defend_item;
pub mod donner;
pub mod hp;
pub mod leaderboard;
pub mod memorial;
pub mod no_taunts;
pub mod prank;
pub mod potion;
pub mod profil;
pub mod refuser;
pub mod repos;
pub mod reset_stats;
pub mod resume;
pub mod shop_cmd;
pub mod taunts_channel;
pub mod tout_ou_rien;
pub mod train;
pub mod voler;

use serenity::builder::CreateCommand;

/// Commandes Coup de Coude — version "fun & simple". Le meta-jeu lourd
/// (braquage/prison, ecosysteme anti-vol, guerre sociale, prestige/ultimate,
/// paris, saisons, maledictions/sabotage) a ete retire des commandes ; les
/// mecaniques internes au combat restent gerees par le moteur.
pub fn all() -> Vec<CreateCommand> {
    vec![
        // Coeur du combat
        coude::register(),
        coude_amical::register(),
        classe::register(),
        train::register(),
        profil::register(),
        leaderboard::register(),
        // PV
        hp::register(),
        repos::register(),
        potion::register(),
        // Boutique + economie simple
        shop_cmd::register(),
        voler::register(),
        donner::register(),
        tout_ou_rien::register(),
        memorial::register(),
        cagnotte::register(),
        // Fun / social light
        prank::register(),
        // Utilitaires
        resume::register(),
        reset_stats::register(),
        no_taunts::register(),
        taunts_channel::register(),
        aide::register(),
    ]
}
