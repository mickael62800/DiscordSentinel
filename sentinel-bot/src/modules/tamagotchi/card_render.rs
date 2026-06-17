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
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
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
fn state_letter(status: &str, hunger: i32, happiness: i32, energy: i32) -> &'static str {
    if status == "sick" {
        return "m";
    }
    if energy <= 25 {
        return "z";
    }
    if hunger <= 25 || happiness <= 25 {
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
        state_letter(&d.status, d.hunger, d.happiness, d.energy),
    )
}

/// Charge le sprite depuis `TAMAGOTCHI_SPRITES_DIR` (defaut: `images`) et le
/// renvoie en base64 (embarquement SVG). `None` si le fichier est absent ->
/// le rendu retombe sur le placeholder.
fn load_sprite_b64(d: &CardData) -> Option<String> {
    use base64::Engine;
    let dir = std::env::var("TAMAGOTCHI_SPRITES_DIR").unwrap_or_else(|_| "images".to_string());
    let path = std::path::Path::new(&dir).join(format!("{}.png", sprite_filename(d)));
    let bytes = std::fs::read(&path).ok()?;
    Some(base64::engine::general_purpose::STANDARD.encode(bytes))
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
    let w = 880.0;
    let h = 250.0;

    // Couleurs.
    let bg = "#232838";
    let track = "#3a4055";
    let grey = "#8b93a7";
    let white = "#ffffff";
    let accent = "#e8a87c";
    let gold = "#f1c40f";
    let green = "#5fd17a";

    let initial = d.name.chars().next().unwrap_or('?').to_uppercase().to_string();

    let status_txt = match d.status.as_str() {
        "sick" => "· 🤒 malade",
        "dead" => "· 🪦 mort",
        _ => "",
    };
    let spec = d.specialization.clone().unwrap_or_default();
    let subtitle = if spec.is_empty() {
        format!("{} · {} jours {}", esc(&d.species_label), d.age_days, status_txt)
    } else {
        format!("{} ({}) · {} jours {}", esc(&d.species_label), esc(&spec), d.age_days, status_txt)
    };

    // Barre XP.
    let xp_pct = if d.xp_for_level > 0 {
        (d.xp_in_level as f32 / d.xp_for_level as f32).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let xp_fill_w = 520.0 * xp_pct;

    // Lignes de jauges (label, valeur, couleur).
    let gauge_bar = |y: f32, label: &str, value: i32, color: &str| -> String {
        format!(
            r##"
            <text x="290" y="{ty}" font-family="DejaVu Sans" font-weight="bold" font-size="15" fill="{white}">{label}</text>
            <rect x="395" y="{by}" width="160" height="16" rx="8" fill="{track}"/>
            <rect x="395" y="{by}" width="{fw}" height="16" rx="8" fill="{color}"/>
            <text x="565" y="{ty}" font-family="DejaVu Sans" font-weight="bold" font-size="15" fill="{white}">{value}</text>
            "##,
            ty = y + 13.0,
            by = y,
            fw = bar_w(value, 160.0),
        )
    };

    // Lignes combat (dot, label, valeur, couleur).
    let combat_row = |y: f32, dot: &str, label: &str, value: i32| -> String {
        format!(
            r##"
            <circle cx="605" cy="{cy}" r="5" fill="{dot}"/>
            <text x="618" y="{ty}" font-family="DejaVu Sans" font-weight="bold" font-size="14" fill="{dot}">{label}</text>
            <text x="845" y="{ty}" text-anchor="end" font-family="DejaVu Sans" font-weight="bold" font-size="16" fill="{white}">{value}</text>
            "##,
            cy = y + 8.0,
            ty = y + 13.0,
        )
    };

    // Avatar : pierre tombale si mort ; sinon sprite d'evolution si dispo ;
    // sinon placeholder (cercle + initiale).
    let avatar_block = if d.status == "dead" {
        dead_avatar_svg()
    } else {
        match load_sprite_b64(d) {
        Some(b64) => format!(
            r##"<defs><clipPath id="avclip"><circle cx="135" cy="125" r="84"/></clipPath></defs>
  <image x="51" y="41" width="168" height="168" href="data:image/png;base64,{b64}" clip-path="url(#avclip)" preserveAspectRatio="xMidYMid slice"/>"##
        ),
        None => format!(
            r##"<circle cx="135" cy="125" r="84" fill="#{species_color}"/>
  <text x="135" y="160" text-anchor="middle" font-family="DejaVu Sans" font-weight="bold" font-size="90" fill="#ffffff" opacity="0.9">{initial}</text>"##,
            species_color = d.species_color,
            initial = esc(&initial),
        ),
        }
    };

    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">
  <rect x="0" y="0" width="{w}" height="{h}" rx="22" fill="#1a1d28"/>
  <rect x="8" y="8" width="{inw}" height="{inh}" rx="18" fill="{bg}"/>
  <rect x="14" y="22" width="7" height="206" rx="3.5" fill="{accent}"/>

  <!-- Avatar -->
  <circle cx="135" cy="125" r="92" fill="#2c3144" stroke="{accent}" stroke-width="3"/>
  {avatar_block}

  <!-- En-tete -->
  <text x="290" y="48" font-family="DejaVu Sans" font-weight="bold" font-size="30" fill="{white}">{name}</text>
  <rect x="760" y="22" width="100" height="38" rx="10" fill="#a98467"/>
  <text x="810" y="47" text-anchor="middle" font-family="DejaVu Sans" font-weight="bold" font-size="17" fill="#2a2118">Niv. {level}</text>
  <text x="290" y="74" font-family="DejaVu Sans" font-size="14" fill="{grey}">{subtitle}</text>

  <!-- Barre XP -->
  <rect x="290" y="92" width="520" height="7" rx="3.5" fill="{track}"/>
  <rect x="290" y="92" width="{xp_fill_w}" height="7" rx="3.5" fill="{accent}"/>
  <text x="290" y="118" font-family="DejaVu Sans" font-size="12" fill="{grey}">XP {xp_in}/{xp_for}</text>

  <!-- Jauges -->
  {g_faim}
  {g_bonheur}
  {g_energie}

  <!-- Combat -->
  <text x="595" y="118" font-family="DejaVu Sans" font-weight="bold" font-size="13" fill="{grey}" letter-spacing="1">COMBAT</text>
  {c_force}
  {c_vit}
  {c_agi}
  <line x1="595" y1="205" x2="855" y2="205" stroke="{track}" stroke-width="1"/>
  <text x="595" y="222" font-family="DejaVu Sans" font-size="13" fill="{grey}">📍 ELO {elo} ({wins}V/{losses}D)</text>

  <!-- Coins -->
  <text x="440" y="240" font-family="DejaVu Sans" font-weight="bold" font-size="15" fill="{gold}">🪙 {coins}</text>
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
        g_faim = gauge_bar(132.0, "FAIM", d.hunger, gold),
        g_bonheur = gauge_bar(162.0, "BONHEUR", d.happiness, green),
        g_energie = gauge_bar(192.0, "ÉNERGIE", d.energy, gold),
        c_force = combat_row(132.0, "#e74c3c", "FORCE", d.str_),
        c_vit = combat_row(162.0, "#5b8def", "VITALITÉ", d.vit),
        c_agi = combat_row(192.0, "#4cd07d", "AGILITÉ", d.agi),
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
    let mut opt = usvg::Options::default();
    opt.fontdb = fontdb();
    let tree = match usvg::Tree::from_str(&svg, &opt) {
        Ok(t) => t,
        Err(e) => {
            warn!(error = %e, "Echec parse SVG carte tamagotchi");
            return None;
        }
    };
    let size = tree.size().to_int_size();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size.width(), size.height())?;
    resvg::render(&tree, resvg::tiny_skia::Transform::identity(), &mut pixmap.as_mut());
    pixmap.encode_png().ok()
}
