//! Embeds Discord pour la Roue du Destin (repris de l'ancien module
//! `sentinel-bot/src/modules/wheel/embeds.rs`).

use serenity::all::CreateEmbed;
use serenity::all::CreateEmbedFooter;

use crate::api_client::WheelSpinResponse;

/// Couleur en fonction du type de resultat.
fn color_for(payout: i64, is_memorable: bool) -> u32 {
    if is_memorable && payout > 0 {
        return 0xf1c40f; // or
    }
    if is_memorable && payout < 0 {
        return 0x8b0000; // rouge sombre apocalypse
    }
    if payout > 0 {
        return 0x2ecc71; // vert
    }
    if payout < 0 {
        return 0xe74c3c; // rouge
    }
    0x95a5a6 // gris (blanche)
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

/// Embed pour signaler une erreur (daily deja claim, API down...).
pub fn build_error_embed(message: &str) -> CreateEmbed {
    CreateEmbed::new()
        .title("\u{1f300} Roue du Destin")
        .description(message)
        .color(0xed4245)
}
