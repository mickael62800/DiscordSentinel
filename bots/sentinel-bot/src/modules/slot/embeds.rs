//! Embeds Discord pour les resultats de spin (loss / refund / win / jackpot).

use serenity::all::{CreateEmbed, CreateEmbedFooter};

use super::api_client::SpinResponse;

const COLOR_LOSS: u32 = 0x95a5a6;     // gris
const COLOR_REFUND: u32 = 0x3498db;   // bleu
const COLOR_WIN: u32 = 0x2ecc71;      // vert
const COLOR_JACKPOT: u32 = 0xf1c40f;  // or

/// Construit l embed du resultat d un spin. Format : 3 emojis geants au
/// centre, puis details mise/gain/multiplier/balance.
pub fn build_spin_result_embed(resp: &SpinResponse, username: &str) -> CreateEmbed {
    let symbols_line = format!("# {}", resp.symbols.join(" \u{2003} "));
    let (title, color) = if resp.is_jackpot {
        ("\u{1f3b0} JACKPOT \u{1f3b0}", COLOR_JACKPOT)
    } else if resp.payout > resp.mise {
        ("\u{1f389} Gagne !", COLOR_WIN)
    } else if resp.payout == resp.mise && resp.payout > 0 {
        ("\u{1f504} 2 identiques — mise rendue", COLOR_REFUND)
    } else {
        ("\u{1f614} Perdu", COLOR_LOSS)
    };

    let net = resp.payout - resp.mise;
    let net_str = if net >= 0 { format!("+{net}") } else { format!("{net}") };

    let mut embed = CreateEmbed::new()
        .title(title)
        .description(symbols_line)
        .color(color)
        .field("Mise", format!("{} coins", resp.mise), true)
        .field("Payout", format!("{} coins", resp.payout), true)
        .field("Solde", format!("{} coins", resp.balance_after), true);

    if resp.multiplier > 0.0 && !resp.is_jackpot {
        embed = embed.field("Multiplicateur", format!("x{:.1}", resp.multiplier), true);
    }
    embed = embed.field("Net", format!("{} coins", net_str), true);

    if resp.is_free {
        embed = embed.field("Type", "\u{1f381} Daily bonus (gratuit)".to_string(), true);
    }

    if resp.is_jackpot {
        embed = embed.footer(CreateEmbedFooter::new(format!(
            "Pool jackpot reset a {} coins. {}",
            resp.jackpot_pool_after, username
        )));
    } else {
        embed = embed.footer(CreateEmbedFooter::new(format!(
            "Pool jackpot actuel : {} coins | {}",
            resp.jackpot_pool_after, username
        )));
    }
    embed
}

/// Embed pour signaler une erreur (mise hors borne, cooldown, etc.).
pub fn build_error_embed(message: &str) -> CreateEmbed {
    CreateEmbed::new()
        .title("\u{1f3b0} Slot")
        .description(message)
        .color(0xed4245)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_loss() -> SpinResponse {
        SpinResponse {
            spin_id: "x".into(),
            symbols: vec!["🍒".into(), "🍋".into(), "🍊".into()],
            mise: 50,
            payout: 0,
            multiplier: 0.0,
            is_jackpot: false,
            is_free: false,
            jackpot_pool_after: 1000,
            balance_after: 100,
            triggered_taunts: vec![],
        }
    }

    fn sample_jackpot() -> SpinResponse {
        SpinResponse {
            spin_id: "x".into(),
            symbols: vec!["7️⃣".into(), "7️⃣".into(), "7️⃣".into()],
            mise: 100,
            payout: 100_000,
            multiplier: 100.0,
            is_jackpot: true,
            is_free: false,
            jackpot_pool_after: 1000,
            balance_after: 100_000,
            triggered_taunts: vec![],
        }
    }

    fn sample_refund() -> SpinResponse {
        SpinResponse {
            spin_id: "x".into(),
            symbols: vec!["🍒".into(), "🍒".into(), "🍋".into()],
            mise: 50,
            payout: 50,
            multiplier: 1.0,
            is_jackpot: false,
            is_free: false,
            jackpot_pool_after: 1000,
            balance_after: 100,
            triggered_taunts: vec![],
        }
    }

    fn sample_win_three_of_a_kind() -> SpinResponse {
        SpinResponse {
            spin_id: "x".into(),
            symbols: vec!["🔔".into(), "🔔".into(), "🔔".into()],
            mise: 100,
            payout: 1200,
            multiplier: 12.0,
            is_jackpot: false,
            is_free: false,
            jackpot_pool_after: 1010,
            balance_after: 1200,
            triggered_taunts: vec![],
        }
    }

    fn embed_to_string(e: &CreateEmbed) -> String {
        // CreateEmbed n a pas de getter public sur title/description en direct :
        // on serialise en JSON pour les tests (pratique courant Serenity).
        serde_json::to_string(e).unwrap()
    }

    #[test]
    fn loss_embed_contains_perdu_label() {
        let r = sample_loss();
        let s = embed_to_string(&build_spin_result_embed(&r, "Alice"));
        assert!(s.contains("Perdu"));
    }

    #[test]
    fn jackpot_embed_contains_jackpot_label() {
        let r = sample_jackpot();
        let s = embed_to_string(&build_spin_result_embed(&r, "Alice"));
        assert!(s.contains("JACKPOT"));
    }

    #[test]
    fn refund_embed_contains_refund_label() {
        let r = sample_refund();
        let s = embed_to_string(&build_spin_result_embed(&r, "Alice"));
        assert!(s.contains("mise rendue") || s.contains("identiques"));
    }

    #[test]
    fn win_embed_contains_gagne_and_multiplier() {
        let r = sample_win_three_of_a_kind();
        let s = embed_to_string(&build_spin_result_embed(&r, "Alice"));
        assert!(s.contains("Gagne"));
        assert!(s.contains("x12"));
    }

    #[test]
    fn embed_shows_net_negative_on_loss() {
        let r = sample_loss();
        let s = embed_to_string(&build_spin_result_embed(&r, "Alice"));
        // mise 50 - payout 0 = -50
        assert!(s.contains("-50"));
    }

    #[test]
    fn embed_shows_net_positive_on_win() {
        let r = sample_win_three_of_a_kind();
        let s = embed_to_string(&build_spin_result_embed(&r, "Alice"));
        // payout 1200 - mise 100 = +1100
        assert!(s.contains("+1100"));
    }

    #[test]
    fn daily_embed_contains_daily_field() {
        let mut r = sample_refund();
        r.is_free = true;
        let s = embed_to_string(&build_spin_result_embed(&r, "Alice"));
        assert!(s.to_lowercase().contains("daily"));
    }

    #[test]
    fn error_embed_contains_message() {
        let s = embed_to_string(&build_error_embed("Cooldown actif"));
        assert!(s.contains("Cooldown"));
    }
}
