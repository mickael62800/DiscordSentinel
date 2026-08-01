use super::*;

// ── Economie ──

/// Le defaut ne doit RIEN changer : c'est la condition pour introduire une
/// configuration sur un jeu deja en cours.
#[test]
fn multiplicateur_neutre_ne_touche_pas_au_gain() {
    let c = EconomyConfig::default();
    assert_eq!(c.wheel_payout_multiplier, 100);
    assert_eq!(c.apply_payout(500), 500);
    assert_eq!(c.apply_payout(-2000), -2000);
}

#[test]
fn multiplicateur_double_les_gains_et_les_pertes() {
    let c = EconomyConfig {
        wheel_payout_multiplier: 200,
        ..Default::default()
    };
    assert_eq!(c.apply_payout(500), 1000);
    // Une perte multipliee reste une perte.
    assert_eq!(c.apply_payout(-500), -1000);
}

/// L'ordre des operations compte : diviser d'abord ecraserait les petits
/// montants a zero.
#[test]
fn multiplicateur_n_ecrase_pas_les_petits_montants() {
    let c = EconomyConfig {
        wheel_payout_multiplier: 120,
        ..Default::default()
    };
    assert_eq!(c.apply_payout(50), 60);
}

#[test]
fn frais_de_transfert_nuls_par_defaut() {
    assert_eq!(EconomyConfig::default().transfer_fee(1000), 0);
}

/// Arrondi vers le BAS : prelever plus que le pourcentage annonce serait une
/// mauvaise surprise.
#[test]
fn frais_arrondis_vers_le_bas() {
    let c = EconomyConfig {
        transfer_fee_pct: 5,
        ..Default::default()
    };
    assert_eq!(c.transfer_fee(199), 9); // 9.95 -> 9
}

#[test]
fn transfert_dans_les_bornes_est_accepte() {
    let c = EconomyConfig {
        transfer_min: 10,
        transfer_max: 100,
        ..Default::default()
    };
    assert!(c.validate_transfer(50).is_ok());
    assert!(c.validate_transfer(10).is_ok());
    assert!(c.validate_transfer(100).is_ok());
}

#[test]
fn transfert_hors_bornes_est_refuse() {
    let c = EconomyConfig {
        transfer_min: 10,
        transfer_max: 100,
        ..Default::default()
    };
    assert!(c.validate_transfer(9).is_err());
    assert!(c.validate_transfer(101).is_err());
}

/// 0 signifie « pas de plafond », pas « plafond a zero » — sans quoi tout
/// transfert serait refuse.
#[test]
fn maximum_a_zero_signifie_aucun_plafond() {
    let c = EconomyConfig {
        transfer_max: 0,
        ..Default::default()
    };
    assert!(c.validate_transfer(1_000_000).is_ok());
}

#[test]
fn transferts_desactives_refusent_tout() {
    let c = EconomyConfig {
        transfer_enabled: false,
        ..Default::default()
    };
    assert!(c.validate_transfer(50).is_err());
}

// ── Coup de Coude ──

/// Les defauts reproduisent les constantes historiques du service de vol.
#[test]
fn defauts_du_vol_reproduisent_le_comportement_historique() {
    let c = CoudeConfig::default();
    assert_eq!(c.steal_chance(false), 30);
    assert_eq!(c.steal_chance(true), 50);
    // victim/5 == 20 %
    assert_eq!(c.steal_gain(100), 20);
    // thief * 15 / 100
    assert_eq!(c.steal_penalty(100), 15);
}

/// Ni 0 ni 100 : le vol doit garder une part de risque des deux cotes.
#[test]
fn chance_de_vol_reste_dans_des_bornes_jouables() {
    let c = CoudeConfig {
        steal_success_pct: 0,
        steal_success_pct_fourbe: 100,
        ..Default::default()
    };
    assert_eq!(c.steal_chance(false), 1);
    assert_eq!(c.steal_chance(true), 99);
}

/// Un vol reussi qui ne rapporte rien serait indistinguable d'un echec.
#[test]
fn un_vol_reussi_rapporte_toujours_au_moins_un_coin() {
    let c = CoudeConfig::default();
    assert_eq!(c.steal_gain(1), 1);
    assert_eq!(c.steal_gain(0), 1);
}

#[test]
fn penalite_d_echec_est_toujours_d_au_moins_un_coin() {
    assert_eq!(CoudeConfig::default().steal_penalty(0), 1);
}

#[test]
fn experience_de_combat_inclut_le_bonus_d_outsider() {
    let c = CoudeConfig::default();
    assert_eq!(c.combat_xp(false), 10);
    assert_eq!(c.combat_xp(true), 15);
}

#[test]
fn mise_dans_les_bornes_est_acceptee() {
    let c = CoudeConfig {
        combat_mise_min: 10,
        combat_mise_max: 500,
        ..Default::default()
    };
    assert!(c.validate_mise(10).is_ok());
    assert!(c.validate_mise(500).is_ok());
    assert!(c.validate_mise(9).is_err());
    assert!(c.validate_mise(501).is_err());
}

#[test]
fn mise_maximum_a_zero_signifie_aucun_plafond() {
    let c = CoudeConfig {
        combat_mise_max: 0,
        ..Default::default()
    };
    assert!(c.validate_mise(1_000_000).is_ok());
}

// ── Lecture des valeurs stockees ──

fn kv(paires: &[(&str, &str)]) -> Vec<BotGuildConfig> {
    paires
        .iter()
        .map(|(k, v)| BotGuildConfig {
            id: uuid::Uuid::nil(),
            guild_id: "g".into(),
            bot_name: "nexus-coude".into(),
            config_key: (*k).into(),
            config_value: (*v).into(),
            updated_at: chrono::Utc::now(),
        })
        .collect()
}

#[test]
fn valeur_stockee_remplace_le_defaut() {
    let items = kv(&[("steal_success_pct", "75")]);
    assert_eq!(n(&items, "steal_success_pct", 30_u32), 75);
}

/// Une saisie erronee ne doit pas rendre le jeu injouable.
#[test]
fn valeur_illisible_retombe_sur_le_defaut() {
    let items = kv(&[("steal_success_pct", "beaucoup")]);
    assert_eq!(n(&items, "steal_success_pct", 30_u32), 30);
}

#[test]
fn cle_absente_retombe_sur_le_defaut() {
    assert_eq!(n(&kv(&[]), "steal_success_pct", 30_u32), 30);
}

#[test]
fn booleen_accepte_les_deux_ecritures() {
    assert!(b(&kv(&[("steal_enabled", "true")]), "steal_enabled", false));
    assert!(b(&kv(&[("steal_enabled", "1")]), "steal_enabled", false));
    assert!(!b(&kv(&[("steal_enabled", "false")]), "steal_enabled", true));
    assert!(!b(&kv(&[("steal_enabled", "0")]), "steal_enabled", true));
}

#[test]
fn booleen_illisible_retombe_sur_le_defaut() {
    assert!(b(&kv(&[("steal_enabled", "oui")]), "steal_enabled", true));
}
