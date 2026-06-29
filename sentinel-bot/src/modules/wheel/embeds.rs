//! Embeds Discord pour la Roue du Destin.

use serenity::all::{CreateEmbed, CreateEmbedFooter};

use super::api_client::WheelSpinResponse;

/// Couleur en fonction du type de resultat.
fn color_for(payout: i64, is_memorable: bool) -> u32 {
    if is_memorable && payout > 0 {
        return 0xf1c40f;
    } // or
    if is_memorable && payout < 0 {
        return 0x8b0000;
    } // rouge sombre apocalypse
    if payout > 0 {
        return 0x2ecc71;
    } // vert
    if payout < 0 {
        return 0xe74c3c;
    } // rouge
    0x95a5a6 // gris (blanche)
}

/// Embed du spin pendant l animation (pas encore revele).
pub fn build_spinning_embed(username: &str) -> CreateEmbed {
    CreateEmbed::new()
        .title("\u{1f300} La Roue du Destin tourne...")
        .description(format!(
            "🎲 La roue spin pour **{}** !\n\n# 🪙 . . . 🪙 . . . 🪙\n\n*Tic... tic... tic...*",
            username
        ))
        .color(0xf1c40f)
        .footer(CreateEmbedFooter::new("Le destin se decide..."))
}

/// Embed final avec le resultat.
pub fn build_result_embed(resp: &WheelSpinResponse, username: &str) -> CreateEmbed {
    let net_str = if resp.payout > 0 {
        format!("+{}", resp.payout)
    } else {
        resp.payout.to_string()
    };
    let title = if resp.is_memorable {
        format!("\u{1f300} {} a tire... LE DESTIN PARLE !", username)
    } else {
        format!("\u{1f300} {} a tire la Roue", username)
    };

    let mut embed = CreateEmbed::new()
        .title(title)
        .description(format!("# {}", resp.case_label))
        .color(color_for(resp.payout, resp.is_memorable))
        .field("Gain", format!("{} coins", net_str), true)
        .field("Solde", format!("{} coins", resp.balance_after), true);

    if resp.is_memorable && resp.payout > 0 {
        embed = embed.footer(CreateEmbedFooter::new(format!(
            "🎉 GROS COUP pour {} ! Reviens demain pour ton prochain spin.",
            username
        )));
    } else if resp.is_memorable && resp.payout < 0 {
        embed = embed.footer(CreateEmbedFooter::new(format!(
            "💀 Le destin a frappe fort. Reviens demain {}.",
            username
        )));
    } else {
        embed = embed.footer(CreateEmbedFooter::new(format!(
            "Reviens demain pour ton prochain spin, {}",
            username
        )));
    }
    embed
}

/// Embed pour signaler une erreur.
pub fn build_error_embed(message: &str) -> CreateEmbed {
    CreateEmbed::new()
        .title("\u{1f300} Roue du Destin")
        .description(message)
        .color(0xed4245)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn json_str(e: &CreateEmbed) -> String {
        serde_json::to_string(e).unwrap()
    }

    fn sample_jackpot() -> WheelSpinResponse {
        WheelSpinResponse {
            spin_id: "x".into(),
            case_key: "jackpot".into(),
            case_label: "🎰 Jackpot — +5000c".into(),
            payout: 5000,
            balance_after: 6000,
            is_memorable: true,
            triggered_taunts: vec![],
        }
    }

    fn sample_blanche() -> WheelSpinResponse {
        WheelSpinResponse {
            spin_id: "x".into(),
            case_key: "blanche".into(),
            case_label: "🌀 Blanche — Rien.".into(),
            payout: 0,
            balance_after: 500,
            is_memorable: false,
            triggered_taunts: vec![],
        }
    }

    fn sample_ruine() -> WheelSpinResponse {
        WheelSpinResponse {
            spin_id: "x".into(),
            case_key: "ruine".into(),
            case_label: "💀 Ruine — -500c".into(),
            payout: -500,
            balance_after: 100,
            is_memorable: false,
            triggered_taunts: vec![],
        }
    }

    #[test]
    fn spinning_embed_contains_username() {
        let s = json_str(&build_spinning_embed("Alice"));
        assert!(s.contains("Alice"));
        assert!(s.contains("tourne"));
    }

    #[test]
    fn jackpot_embed_is_gold_and_memorable() {
        let e = build_result_embed(&sample_jackpot(), "Bob");
        let s = json_str(&e);
        // 0xf1c40f = 15844367 en decimal
        assert!(s.contains("15844367"));
        assert!(s.contains("DESTIN PARLE"));
        assert!(s.contains("+5000"));
        assert!(s.contains("GROS COUP"));
    }

    #[test]
    fn blanche_embed_is_grey_neutral() {
        let e = build_result_embed(&sample_blanche(), "Alice");
        let s = json_str(&e);
        // La couleur 0x95a5a6 est testee via la fonction directement
        // (color_blanche_zero ci-dessous). Ici on verifie la structure
        // de l embed.
        assert!(!s.contains("DESTIN PARLE"));
        assert!(s.contains("Blanche"));
        assert!(s.contains("0 coins"));
    }

    #[test]
    fn ruine_embed_is_red_negative() {
        let e = build_result_embed(&sample_ruine(), "Alice");
        let s = json_str(&e);
        // 0xe74c3c = 15158332
        assert!(s.contains("15158332"));
        assert!(s.contains("-500"));
    }

    #[test]
    fn error_embed_contains_message() {
        let s = json_str(&build_error_embed("Deja tire aujourd hui"));
        assert!(s.contains("Deja tire"));
    }

    #[test]
    fn color_jackpot_memorable_positive() {
        assert_eq!(color_for(5000, true), 0xf1c40f);
    }

    #[test]
    fn color_bombe_memorable_negative() {
        assert_eq!(color_for(-2000, true), 0x8b0000);
    }

    #[test]
    fn color_normal_gain() {
        assert_eq!(color_for(200, false), 0x2ecc71);
    }

    #[test]
    fn color_normal_loss() {
        assert_eq!(color_for(-500, false), 0xe74c3c);
    }

    #[test]
    fn color_blanche_zero() {
        assert_eq!(color_for(0, false), 0x95a5a6);
    }
}
