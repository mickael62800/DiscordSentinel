//! Rendu de la carte compagnon en image PNG (SVG -> PNG via resvg).
//!
//! On dessine un SVG au layout fixe (fond arrondi, avatar rond, badge niveau,
//! barres de jauges colorees, colonne combat) puis on le rasterise. La police
//! est chargee depuis les fonts systeme (installees dans l'image Docker).
//! L'avatar est pour l'instant un placeholder (cercle colore + initiale) ;
//! il sera remplace par les illustrations d'especes plus tard.

use std::sync::Arc;

use once_cell::sync::OnceCell;
use resvg::usvg;
use tracing::warn;

/// Donnees affichees sur la carte.
pub struct CardData {
    pub name: String,
    pub species_label: String,
    pub specialization: Option<String>,
    pub age_days: i64,
    pub level: i32,
    pub xp_in_level: i64,
    pub xp_for_level: i64,
    pub hunger: i32,
    pub happiness: i32,
    pub energy: i32,
    pub str_: i32,
    pub vit: i32,
    pub agi: i32,
    pub elo: i32,
    pub wins: i32,
    pub losses: i32,
    pub coins: i64,
    pub status: String,
    /// Couleur d'accent de l'espece (hex sans #), pour le placeholder avatar.
    pub species_color: String,
    /// Slug de l'espece (ex. "loup") pour choisir le sprite d'evolution.
    pub species_slug: String,
    /// Seuil d'energie (<=) sous lequel le sprite affiche l'etat fatigue.
    /// Reglable par serveur (`sprite_tired_energy_threshold`, defaut 25),
    /// clampe [0, 100] a la construction.
    pub sprite_tired_energy_threshold: i32,
    /// Seuil de faim/bonheur (<=) sous lequel le sprite affiche l'etat
    /// affame/mecontent (`sprite_unhappy_stat_threshold`, defaut 25),
    /// clampe [0, 100] a la construction.
    pub sprite_unhappy_stat_threshold: i32,
}

static FONTDB: OnceCell<Arc<usvg::fontdb::Database>> = OnceCell::new();

fn fontdb() -> Arc<usvg::fontdb::Database> {
    FONTDB
        .get_or_init(|| {
            let mut db = usvg::fontdb::Database::new();
            db.load_system_fonts();
            Arc::new(db)
        })
        .clone()
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Largeur remplie d'une barre (sur `max_w`) pour une jauge 0-100.
fn bar_w(value: i32, max_w: f32) -> f32 {
    (value.clamp(0, 100) as f32 / 100.0) * max_w
}

/// Stade d'evolution derive du niveau (cf. docs/TAMAGOTCHI_EVOLUTIONS_PROMPTS.md).
pub(super) fn stage_from_level(level: i32) -> &'static str {
    match level {
        ..=4 => "bebe",
        5..=14 => "jeune",
        15..=29 => "adulte",
        _ => "vieux",
    }
}

/// Libelle affichable d'un stade (pour le message d'evolution).
pub(super) fn stage_label(level: i32) -> &'static str {
    match stage_from_level(level) {
        "bebe" => "bébé",
        "jeune" => "jeune",
        "adulte" => "adulte",
        _ => "vieux",
    }
}

/// Lettre d'espece (1er segment du nom de fichier). loup = l ; les autres
/// seront ajoutees a mesure que les sprites arrivent (lapin != loup : 'p').
fn species_letter(slug: &str) -> &'static str {
    match slug {
        "loup" => "l",
        "sanglier" => "s",
        "renard" => "r",
        "tortue" => "t",
        "ours" => "o",
        "lapin" => "p", // 'l' est pris par loup -> a confirmer quand le lapin arrive
        _ => "x",
    }
}

/// Lettre de stade (2e segment) : b=bebe, j=jeune, a=adulte, v=vieux.
fn stage_letter(level: i32) -> &'static str {
    match stage_from_level(level) {
        "bebe" => "b",
        "jeune" => "j",
        "adulte" => "a",
        _ => "v",
    }
}

/// Lettre d'etat (3e segment) : a=affame, c=content, m=malade, z=dodo (fatigue).
/// Priorite : malade > dodo > affame > content.
fn state_letter(
    status: &str,
    hunger: i32,
    happiness: i32,
    energy: i32,
    tired_energy_threshold: i32,
    unhappy_stat_threshold: i32,
) -> &'static str {
    if status == "sick" {
        return "m";
    }
    if energy <= tired_energy_threshold {
        return "z";
    }
    if hunger <= unhappy_stat_threshold || happiness <= unhappy_stat_threshold {
        return "a";
    }
    "c"
}

/// Nom de fichier du sprite (sans extension) : `{espece}_{stade}_{etat}`
/// (ex. `l_a_c` = loup adulte content). Mort -> logo partage `mort`.
fn sprite_filename(d: &CardData) -> String {
    if d.status == "dead" {
        return "mort".to_string();
    }
    format!(
        "{}_{}_{}",
        species_letter(&d.species_slug),
        stage_letter(d.level),
        state_letter(
            &d.status,
            d.hunger,
            d.happiness,
            d.energy,
            d.sprite_tired_energy_threshold,
            d.sprite_unhappy_stat_threshold,
        ),
    )
}

/// Charge le sprite depuis le dossier de sprites (cf. `sprites_dir`) et le
/// renvoie en base64 (embarquement SVG). `None` si le fichier est absent ->
/// le rendu retombe sur le placeholder.
fn load_sprite_b64(d: &CardData) -> Option<String> {
    use base64::Engine;
    let file = format!("{}.png", sprite_filename(d));
    let bytes = std::fs::read(sprites_dir().join(&file)).ok()?;
    Some(base64::engine::general_purpose::STANDARD.encode(bytes))
}

/// Racine des sprites. Override via `TAMAGOTCHI_SPRITES_DIR` ; sinon on essaie
/// les emplacements usuels (meme logique que les cartes Blackjack) : cwd local
/// (`cargo run` depuis la racine du workspace) ET conteneur Docker (`/app`).
fn sprites_dir() -> std::path::PathBuf {
    use std::path::PathBuf;
    if let Ok(custom) = std::env::var("TAMAGOTCHI_SPRITES_DIR") {
        return PathBuf::from(custom);
    }
    for candidate in [
        "assets/tamagotchi",
        "sentinel-bot/assets/tamagotchi",
        "/app/assets/tamagotchi",
        "images",
    ] {
        let p = PathBuf::from(candidate);
        if p.is_dir() {
            return p;
        }
    }
    PathBuf::from("assets/tamagotchi")
}

/// Avatar "mort" : une pierre tombale RIP dessinee en SVG (aucun asset requis).
fn dead_avatar_svg() -> String {
    r##"<rect x="93" y="74" width="84" height="118" rx="42" fill="#6b7280"/>
  <rect x="86" y="182" width="98" height="16" rx="5" fill="#4b5563"/>
  <rect x="129" y="96" width="12" height="46" rx="3" fill="#e5e7eb"/>
  <rect x="116" y="110" width="38" height="12" rx="3" fill="#e5e7eb"/>
  <text x="135" y="172" text-anchor="middle" font-family="DejaVu Sans" font-weight="bold" font-size="26" fill="#e5e7eb">RIP</text>"##
        .to_string()
}

/// Construit le SVG de la carte.
fn build_svg(d: &CardData) -> String {
    let w = 900.0;
    let h = 640.0;

    // Couleurs.
    let bg = "#232838";
    let track = "#3a4055";
    let grey = "#8b93a7";
    let white = "#ffffff";
    let accent = "#e8a87c";
    let gold = "#f1c40f";
    let green = "#5fd17a";

    let initial = d
        .name
        .chars()
        .next()
        .unwrap_or('?')
        .to_uppercase()
        .to_string();

    let status_txt = match d.status.as_str() {
        "sick" => "· 🤒 malade",
        "dead" => "· 🪦 mort",
        _ => "",
    };
    let spec = d.specialization.clone().unwrap_or_default();
    let subtitle = if spec.is_empty() {
        format!(
            "{} · {} jours {}",
            esc(&d.species_label),
            d.age_days,
            status_txt
        )
    } else {
        format!(
            "{} ({}) · {} jours {}",
            esc(&d.species_label),
            esc(&spec),
            d.age_days,
            status_txt
        )
    };

    // Barre XP.
    let xp_pct = if d.xp_for_level > 0 {
        (d.xp_in_level as f32 / d.xp_for_level as f32).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let xp_fill_w = 812.0 * xp_pct;

    // Lignes de jauges (label, valeur, couleur). Colonne gauche, sous l'image.
    let gauge_bar = |y: f32, label: &str, value: i32, color: &str| -> String {
        format!(
            r##"
            <text x="44" y="{ty}" font-family="DejaVu Sans" font-weight="bold" font-size="16" fill="{white}">{label}</text>
            <rect x="160" y="{by}" width="240" height="18" rx="9" fill="{track}"/>
            <rect x="160" y="{by}" width="{fw}" height="18" rx="9" fill="{color}"/>
            <text x="412" y="{ty}" font-family="DejaVu Sans" font-weight="bold" font-size="16" fill="{white}">{value}</text>
            "##,
            ty = y + 14.0,
            by = y,
            fw = bar_w(value, 240.0),
        )
    };

    // Lignes combat (dot, label, valeur, couleur). Colonne droite, sous l'image.
    let combat_row = |y: f32, dot: &str, label: &str, value: i32| -> String {
        format!(
            r##"
            <circle cx="500" cy="{cy}" r="5" fill="{dot}"/>
            <text x="514" y="{ty}" font-family="DejaVu Sans" font-weight="bold" font-size="15" fill="{dot}">{label}</text>
            <text x="852" y="{ty}" text-anchor="end" font-family="DejaVu Sans" font-weight="bold" font-size="17" fill="{white}">{value}</text>
            "##,
            cy = y + 9.0,
            ty = y + 14.0,
        )
    };

    // Image de l'animal : centree, ENTIERE (preserveAspectRatio meet, sans clip
    // ni rognage). Pierre tombale si mort ; sinon sprite d'evolution si dispo ;
    // sinon placeholder (cercle + initiale).
    let avatar_block = if d.status == "dead" {
        // Tombstone d'origine recentree/agrandie dans le cadre (centre ~450,290).
        format!(
            r##"<g transform="translate(247.5,86) scale(1.5)">{}</g>"##,
            dead_avatar_svg()
        )
    } else {
        match load_sprite_b64(d) {
            Some(b64) => format!(
                r##"<image x="318" y="158" width="264" height="264" href="data:image/png;base64,{b64}" preserveAspectRatio="xMidYMid meet"/>"##
            ),
            None => format!(
                r##"<circle cx="450" cy="290" r="120" fill="#{species_color}"/>
  <text x="450" y="335" text-anchor="middle" font-family="DejaVu Sans" font-weight="bold" font-size="120" fill="#ffffff" opacity="0.9">{initial}</text>"##,
                species_color = d.species_color,
                initial = esc(&initial),
            ),
        }
    };

    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">
  <rect x="0" y="0" width="{w}" height="{h}" rx="22" fill="#1a1d28"/>
  <rect x="8" y="8" width="{inw}" height="{inh}" rx="18" fill="{bg}"/>
  <rect x="14" y="22" width="7" height="596" rx="3.5" fill="{accent}"/>

  <!-- En-tete -->
  <text x="44" y="58" font-family="DejaVu Sans" font-weight="bold" font-size="32" fill="{white}">{name}</text>
  <rect x="762" y="30" width="108" height="42" rx="11" fill="#a98467"/>
  <text x="816" y="58" text-anchor="middle" font-family="DejaVu Sans" font-weight="bold" font-size="18" fill="#2a2118">Niv. {level}</text>
  <text x="44" y="86" font-family="DejaVu Sans" font-size="15" fill="{grey}">{subtitle}</text>

  <!-- Barre XP -->
  <rect x="44" y="100" width="812" height="8" rx="4" fill="{track}"/>
  <rect x="44" y="100" width="{xp_fill_w}" height="8" rx="4" fill="{accent}"/>
  <text x="44" y="128" font-family="DejaVu Sans" font-size="13" fill="{grey}">XP {xp_in}/{xp_for}</text>

  <!-- Image de l'animal (cadre centre) -->
  <rect x="306" y="146" width="288" height="288" rx="26" fill="#2c3144" stroke="{accent}" stroke-width="3"/>
  {avatar_block}

  <!-- Jauges (colonne gauche, sous l'image) -->
  {g_faim}
  {g_bonheur}
  {g_energie}

  <!-- Combat (colonne droite, sous l'image) -->
  <text x="500" y="476" font-family="DejaVu Sans" font-weight="bold" font-size="14" fill="{grey}" letter-spacing="1">COMBAT</text>
  {c_force}
  {c_vit}
  {c_agi}
  <line x1="500" y1="600" x2="856" y2="600" stroke="{track}" stroke-width="1"/>
  <text x="500" y="624" font-family="DejaVu Sans" font-size="14" fill="{grey}">📍 ELO {elo} ({wins}V/{losses}D)</text>

  <!-- Coins -->
  <text x="44" y="624" font-family="DejaVu Sans" font-weight="bold" font-size="16" fill="{gold}">🪙 {coins}</text>
</svg>"##,
        inw = w - 16.0,
        inh = h - 16.0,
        avatar_block = avatar_block,
        name = esc(&d.name),
        level = d.level,
        subtitle = subtitle,
        xp_fill_w = xp_fill_w,
        xp_in = d.xp_in_level,
        xp_for = d.xp_for_level,
        g_faim = gauge_bar(490.0, "FAIM", d.hunger, gold),
        g_bonheur = gauge_bar(525.0, "BONHEUR", d.happiness, green),
        g_energie = gauge_bar(560.0, "ÉNERGIE", d.energy, gold),
        c_force = combat_row(490.0, "#e74c3c", "FORCE", d.str_),
        c_vit = combat_row(525.0, "#5b8def", "VITALITÉ", d.vit),
        c_agi = combat_row(560.0, "#4cd07d", "AGILITÉ", d.agi),
        elo = d.elo,
        wins = d.wins,
        losses = d.losses,
        coins = d.coins,
    )
}

/// Rend la carte en PNG. Retourne None en cas d'echec (le caller retombe
/// alors sur l'embed texte).
pub fn render_card_png(d: &CardData) -> Option<Vec<u8>> {
    let svg = build_svg(d);
    let opt = usvg::Options {
        fontdb: fontdb(),
        ..Default::default()
    };
    let tree = match usvg::Tree::from_str(&svg, &opt) {
        Ok(t) => t,
        Err(e) => {
            warn!(error = %e, "Echec parse SVG carte tamagotchi");
            return None;
        }
    };
    let size = tree.size().to_int_size();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size.width(), size.height())?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    pixmap.encode_png().ok()
}
