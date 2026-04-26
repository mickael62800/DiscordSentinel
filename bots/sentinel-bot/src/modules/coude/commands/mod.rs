pub mod accepter;
pub mod aide;
pub mod annuler;
pub mod assurance;
pub mod boost_voleur;
pub mod braquage;
pub mod cagnotte;
pub mod classe;
pub mod coalition;
pub mod contribuer_prime;
pub mod coude;
pub mod defend_item;
pub mod donner;
pub mod honneur;
pub mod hp;
pub mod leaderboard;
pub mod maudire;
pub mod memorial;
pub mod no_taunts;
pub mod prank;
pub mod pari;
pub mod potion;
pub mod prestige;
pub mod prime;
pub mod profil;
pub mod protection;
pub mod refuser;
pub mod repos;
pub mod saboter;
pub mod reset_stats;
pub mod resume;
pub mod saison;
pub mod shop_cmd;
pub mod taunts_channel;
pub mod tout_ou_rien;
pub mod train;
pub mod travaux;
pub mod ultimate;
pub mod vendetta;
pub mod voler;

use serenity::builder::CreateCommand;

pub fn all() -> Vec<CreateCommand> {
    vec![
        coude::register(),
        profil::register(),
        shop_cmd::register(),
        prime::register(),
        leaderboard::register(),
        pari::register(),
        potion::register(),
        voler::register(),
        assurance::register(),
        train::register(),
        classe::register(),
        donner::register(),
        hp::register(),
        repos::register(),
        saison::register(),
        reset_stats::register(),
        resume::register(),
        cagnotte::register(),
        protection::register(),
        boost_voleur::register(),
        no_taunts::register(),
        taunts_channel::register(),
        braquage::register(),
        maudire::register(),
        prank::register(),
        aide::register(),
        saboter::register(),
        tout_ou_rien::register(),
        vendetta::register(),
        memorial::register(),
        contribuer_prime::register(),
        honneur::register(),
        coalition::register(),
        ultimate::register(),
        prestige::register(),
        travaux::register(),
    ]
}
