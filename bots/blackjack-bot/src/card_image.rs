//! Composition de plusieurs cartes en une seule image horizontale.
//!
//! Les assets JPG individuels se trouvent dans `bots/blackjack-bot/assets/cards/`
//! au format `{Rank}_{suit}.jpg` (ex : `As_heart.jpg`, `10_club.jpg`,
//! `Jack_spade.jpg`). Le domaine utilise `hearts/diamonds/clubs/spades` au
//! pluriel, on strippe le `s` final ici pour matcher les noms de fichiers.
//!
//! La composition produit un PNG en memoire qu'on envoie comme attachment
//! Discord (`attachment://player.png`) referenced depuis l'embed pour
//! afficher la main directement dans le message.

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use image::imageops::FilterType;
use image::{DynamicImage, GenericImage, ImageFormat, Rgba, RgbaImage};
use once_cell::sync::Lazy;
use tracing::{debug, warn};

use crate::api_client::CardDto;

/// Largeur cible d'une carte dans l'image composite (px).
const CARD_WIDTH: u32 = 160;
/// Hauteur cible (ratio carte standard 2.5x3.5 inches ~ 1:1.4).
const CARD_HEIGHT: u32 = 224;
/// Espace entre cartes (px).
const CARD_GAP: u32 = 12;
/// Marge autour des cartes (px).
const PADDING: u32 = 16;
/// Couleur de fond de la composition (RGBA, vert table de casino).
const BG_COLOR: [u8; 4] = [20, 70, 40, 255];

/// Cache des cartes decodees en memoire pour eviter de relire les JPG a
/// chaque coup. 52 images * ~30 KB = ~1.5 MB max.
static CARD_CACHE: Lazy<Mutex<std::collections::HashMap<String, DynamicImage>>> =
    Lazy::new(|| Mutex::new(std::collections::HashMap::new()));

/// Racine des assets. Configurable via `BJ_ASSETS_DIR` (fallback au chemin
/// relatif attendu dans l'image Docker / en dev).
fn assets_dir() -> PathBuf {
    if let Ok(custom) = std::env::var("BJ_ASSETS_DIR") {
        return PathBuf::from(custom);
    }
    // Chemin relatif au binaire (Docker) ou au workspace (dev).
    for candidate in [
        "assets/cards",
        "bots/blackjack-bot/assets/cards",
        "/app/assets/cards",
    ] {
        let p = PathBuf::from(candidate);
        if p.exists() {
            return p;
        }
    }
    PathBuf::from("assets/cards")
}

/// Nom de fichier pour une carte donnee. Le domaine renvoie les suits au
/// pluriel ("hearts"), les assets sont au singulier ("heart"). On strippe
/// le `s` final et on gere le cas "hidden".
fn card_filename(card: &CardDto) -> Option<String> {
    if card.rank == "hidden" {
        return None;
    }
    let suit = card.suit.trim_end_matches('s');
    Some(format!("{}_{}.jpg", card.rank, suit))
}

/// Charge une carte depuis le cache ou depuis le disque.
fn load_card(card: &CardDto) -> Option<DynamicImage> {
    let filename = card_filename(card)?;
    {
        let cache = CARD_CACHE.lock().ok()?;
        if let Some(img) = cache.get(&filename) {
            return Some(img.clone());
        }
    }
    let path = assets_dir().join(&filename);
    let img = match image::open(&path) {
        Ok(i) => i,
        Err(e) => {
            warn!(path = %path.display(), error = %e, "Carte introuvable");
            return None;
        }
    };
    // Redimensionne a la taille cible pour uniformiser.
    let resized = img.resize_exact(CARD_WIDTH, CARD_HEIGHT, FilterType::Lanczos3);
    if let Ok(mut cache) = CARD_CACHE.lock() {
        cache.insert(filename, resized.clone());
    }
    Some(resized)
}

/// Dessine une carte "dos" (pour la main cachee du dealer).
fn card_back() -> DynamicImage {
    let back_path = assets_dir().join("back.jpg");
    if back_path.exists() {
        if let Ok(img) = image::open(&back_path) {
            return img.resize_exact(CARD_WIDTH, CARD_HEIGHT, FilterType::Lanczos3);
        }
    }
    // Fallback : rectangle bleu fonce avec motif simple.
    let mut img = RgbaImage::from_pixel(CARD_WIDTH, CARD_HEIGHT, Rgba([30, 60, 120, 255]));
    // Bordure blanche
    for x in 0..CARD_WIDTH {
        for y in 0..CARD_HEIGHT {
            let border = x < 4 || y < 4 || x >= CARD_WIDTH - 4 || y >= CARD_HEIGHT - 4;
            if border {
                img.put_pixel(x, y, Rgba([240, 240, 240, 255]));
            }
        }
    }
    DynamicImage::ImageRgba8(img)
}

/// Bande separatrice entre les deux mains (px).
const DIVIDER_HEIGHT: u32 = 3;
/// Espace vertical entre les 2 rangees de cartes (px).
const ROW_GAP: u32 = 24;

/// Compose les 2 mains (dealer en haut, joueur en bas) en une image PNG
/// unique. Retourne les bytes prets a etre envoyes en attachment Discord
/// (`attachment://table.png` dans l'embed).
pub fn render_table(player_hand: &[CardDto], dealer_hand: &[CardDto]) -> Option<Vec<u8>> {
    if player_hand.is_empty() && dealer_hand.is_empty() {
        return None;
    }

    let p_count = player_hand.len().max(1) as u32;
    let d_count = dealer_hand.len().max(1) as u32;
    let max_count = p_count.max(d_count);

    let width =
        PADDING * 2 + max_count * CARD_WIDTH + (max_count.saturating_sub(1)) * CARD_GAP;
    let height = PADDING * 2 + CARD_HEIGHT * 2 + ROW_GAP + DIVIDER_HEIGHT;

    let mut canvas =
        RgbaImage::from_pixel(width, height, Rgba([BG_COLOR[0], BG_COLOR[1], BG_COLOR[2], 255]));

    // Rangee dealer (haut)
    draw_row(&mut canvas, dealer_hand, PADDING);

    // Ligne de separation doree
    let divider_y = PADDING + CARD_HEIGHT + ROW_GAP / 2;
    for x in PADDING..(width - PADDING) {
        for dy in 0..DIVIDER_HEIGHT {
            canvas.put_pixel(x, divider_y + dy, Rgba([241, 196, 15, 200]));
        }
    }

    // Rangee joueur (bas)
    let player_y = PADDING + CARD_HEIGHT + ROW_GAP + DIVIDER_HEIGHT;
    draw_row(&mut canvas, player_hand, player_y);

    let mut buf = Vec::with_capacity(width as usize * height as usize * 3);
    DynamicImage::ImageRgba8(canvas)
        .write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
        .ok()?;
    Some(buf)
}

/// Dessine une rangee de cartes centree horizontalement a la coordonnee y.
fn draw_row(canvas: &mut RgbaImage, hand: &[CardDto], y: u32) {
    if hand.is_empty() {
        return;
    }
    let n = hand.len() as u32;
    let row_width = n * CARD_WIDTH + (n.saturating_sub(1)) * CARD_GAP;
    let start_x = (canvas.width().saturating_sub(row_width)) / 2;

    for (i, card) in hand.iter().enumerate() {
        let x = start_x + (i as u32) * (CARD_WIDTH + CARD_GAP);
        let card_img = if card.rank == "hidden" {
            card_back()
        } else {
            match load_card(card) {
                Some(img) => img,
                None => {
                    debug!(filename = ?card_filename(card), "Carte ignoree");
                    continue;
                }
            }
        };
        let rgba = card_img.to_rgba8();
        canvas.copy_from(&rgba, x, y).ok();
    }
}

/// Compose les cartes d'une main en une image PNG horizontale (utilisee par
/// les builders d'embed qui n'ont besoin que d'une main seule).
#[allow(dead_code)]
pub fn render_hand(hand: &[CardDto]) -> Option<Vec<u8>> {
    if hand.is_empty() {
        return None;
    }
    let n = hand.len() as u32;
    let width = PADDING * 2 + n * CARD_WIDTH + (n.saturating_sub(1)) * CARD_GAP;
    let height = PADDING * 2 + CARD_HEIGHT;

    let mut canvas =
        RgbaImage::from_pixel(width, height, Rgba([BG_COLOR[0], BG_COLOR[1], BG_COLOR[2], 255]));
    draw_row(&mut canvas, hand, PADDING);

    let mut buf = Vec::with_capacity(width as usize * height as usize * 3);
    DynamicImage::ImageRgba8(canvas)
        .write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
        .ok()?;
    Some(buf)
}

/// Force le chargement (et mise en cache) de tous les assets au demarrage.
/// A appeler depuis `main.rs` pour eviter la latence au premier coup.
#[allow(dead_code)]
pub fn preload_cache() {
    let dir = assets_dir();
    debug!(dir = %dir.display(), "Preloading card assets");
    let ranks = [
        "As", "2", "3", "4", "5", "6", "7", "8", "9", "10", "Jack", "Queen", "King",
    ];
    let suits = ["heart", "diamond", "club", "spade"];
    for rank in ranks {
        for suit in suits {
            let fake = CardDto {
                rank: rank.to_string(),
                suit: format!("{suit}s"),
                filename: format!("{rank}_{suit}.jpg"),
            };
            let _ = load_card(&fake);
        }
    }
}

#[allow(dead_code)]
pub fn cards_dir_exists() -> bool {
    let d = assets_dir();
    let exists = d.exists();
    debug!(dir = %d.display(), exists, "Vérification repertoire cartes");
    exists
}

fn _ensure_bounds<P: AsRef<Path>>(_p: P) {
    // Placeholder : si on voulait clipper/ignorer les chemins suspects.
}
