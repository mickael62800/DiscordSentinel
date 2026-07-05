//! Rendu des classements (général / écrit / vocal) en image PNG, composités sur
//! les templates fournis (`assets/leaderboard/top*.png`) via resvg (SVG -> PNG),
//! comme la carte tamagotchi. On place les avatars (cercles) + pseudos + XP aux
//! emplacements du template (podium top 3 + rangs 4-13 sur 2 colonnes).

use std::path::PathBuf;
use std::sync::Arc;

use base64::Engine;
use once_cell::sync::OnceCell;
use resvg::usvg;
use tracing::warn;

/// Catégorie de classement -> fichier template.
#[derive(Clone, Copy)]
pub enum Category {
    General,
    Ecrit,
    Vocal,
}

impl Category {
    pub fn file(self) -> &'static str {
        match self {
            Category::General => "topgeneral.png",
            Category::Ecrit => "topecrit.png",
            Category::Vocal => "topvocal.png",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Category::General => "Général",
            Category::Ecrit => "Écrit",
            Category::Vocal => "Vocal",
        }
    }
}

/// Une entrée du classement (déjà résolue : pseudo + XP + avatar téléchargé).
pub struct LbEntry {
    pub name: String,
    pub xp: i64,
    pub avatar_png: Option<Vec<u8>>,
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

/// Racine des templates de classement (cwd local + conteneur Docker).
fn templates_dir() -> PathBuf {
    for c in [
        "assets/leaderboard",
        "sentinel-bot/assets/leaderboard",
        "/app/assets/leaderboard",
        "imgs",
    ] {
        let p = PathBuf::from(c);
        if p.is_dir() {
            return p;
        }
    }
    PathBuf::from("assets/leaderboard")
}

fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .chars()
        .take(18)
        .collect()
}

/// XP au format abrégé : 12 400 -> 12.4k, 1 500 000 -> 1.5M.
fn fmt_xp(xp: i64) -> String {
    let x = xp.max(0);
    if x >= 1_000_000 {
        format!("{:.1}M", x as f64 / 1_000_000.0)
    } else if x >= 1_000 {
        format!("{:.1}k", x as f64 / 1_000.0)
    } else {
        x.to_string()
    }
}

// ── Coordonnées PAR TEMPLATE (calibrables) ──
// Chaque template est une image distincte (dimensions + positions differentes),
// donc chaque categorie a son propre Layout.

/// Cercle d'avatar : centre (x,y) + rayon.
struct Circle {
    cx: f32,
    cy: f32,
    r: f32,
}

struct Layout {
    /// Dimensions reelles du template (px).
    w: f32,
    h: f32,
    /// Podium : index 0 = rang 1 (centre), 1 = rang 2 (gauche), 2 = rang 3 (droite).
    podium: [Circle; 3],
    /// Y (centre) des 5 lignes de chaque colonne.
    row_ys: [f32; 5],
    row_avatar_r: f32,
    l_avatar_x: f32,
    l_name_x: f32,
    l_xp_x: f32,
    r_avatar_x: f32,
    r_name_x: f32,
    r_xp_x: f32,
}

fn layout_for(cat: Category) -> Layout {
    match cat {
        // topgeneral.png — 1536×1024
        Category::General => Layout {
            w: 1536.0,
            h: 1024.0,
            // Centres exacts detectes via les trous transparents (alpha=0).
            // Rayon un peu > trou : l'avatar remplit le trou, le template masque
            // le surplus (effet encastre).
            podium: [
                Circle { cx: 768.0, cy: 406.0, r: 121.0 }, // #1 centre
                Circle { cx: 420.0, cy: 422.0, r: 103.0 }, // #2 gauche
                Circle { cx: 1080.0, cy: 436.0, r: 96.0 }, // #3 droite
            ],
            row_ys: [656.0, 724.0, 792.0, 860.0, 928.0],
            row_avatar_r: 33.0,
            l_avatar_x: 232.0,
            l_name_x: 278.0,
            l_xp_x: 655.0,
            r_avatar_x: 894.0,
            r_name_x: 938.0,
            r_xp_x: 1315.0,
        },
        // topecrit.png — 1402×1122 (pas d'encadre XP : XP en fin de ligne)
        Category::Ecrit => Layout {
            w: 1402.0,
            h: 1122.0,
            // Centres/rayons mesures sur le template ecrit.
            podium: [
                Circle { cx: 700.0, cy: 463.0, r: 97.0 },  // #1 centre
                Circle { cx: 375.0, cy: 475.0, r: 92.0 },  // #2 gauche
                Circle { cx: 1020.0, cy: 480.0, r: 90.0 }, // #3 droite
            ],
            // Y des lignes mesures sur le template ecrit.
            row_ys: [802.0, 868.0, 931.0, 994.0, 1052.0],
            row_avatar_r: 30.0,
            l_avatar_x: 280.0,
            l_name_x: 324.0,
            l_xp_x: 620.0,
            r_avatar_x: 845.0,
            r_name_x: 889.0,
            r_xp_x: 1200.0,
        },
        // topvocal.png — meme disposition supposee que le general (a confirmer
        // quand le template sera fourni).
        Category::Vocal => Layout {
            w: 1536.0,
            h: 1024.0,
            podium: [
                Circle { cx: 768.0, cy: 406.0, r: 121.0 },
                Circle { cx: 420.0, cy: 422.0, r: 103.0 },
                Circle { cx: 1080.0, cy: 436.0, r: 96.0 },
            ],
            row_ys: [656.0, 724.0, 792.0, 860.0, 928.0],
            row_avatar_r: 33.0,
            l_avatar_x: 232.0,
            l_name_x: 278.0,
            l_xp_x: 655.0,
            r_avatar_x: 894.0,
            r_name_x: 938.0,
            r_xp_x: 1315.0,
        },
    }
}

fn avatar_svg(id: usize, c: &Circle, png_b64: &Option<String>) -> String {
    let Some(data) = png_b64 else {
        return String::new();
    };
    format!(
        "<clipPath id=\"c{id}\"><circle cx=\"{cx}\" cy=\"{cy}\" r=\"{r}\"/></clipPath>\
         <image x=\"{x}\" y=\"{y}\" width=\"{d}\" height=\"{d}\" clip-path=\"url(#c{id})\" \
         preserveAspectRatio=\"xMidYMid slice\" href=\"data:image/png;base64,{data}\"/>",
        cx = c.cx,
        cy = c.cy,
        r = c.r,
        x = c.cx - c.r,
        y = c.cy - c.r,
        d = c.r * 2.0,
    )
}

fn text(x: f32, y: f32, anchor: &str, size: f32, s: &str) -> String {
    // DejaVu Sans : police installee dans l'image Docker (comme la carte tama).
    format!(
        "<text x=\"{x}\" y=\"{y}\" text-anchor=\"{anchor}\" font-family=\"DejaVu Sans\" \
         font-weight=\"bold\" font-size=\"{size}\" fill=\"#ffffff\" \
         stroke=\"#000000\" stroke-width=\"3\" paint-order=\"stroke\">{}</text>",
        esc(s)
    )
}

fn build_svg(template_b64: &str, entries: &[LbEntry], lay: &Layout, template_on_top: bool) -> String {
    let mut avatars = String::new();
    let mut labels = String::new();

    for (i, e) in entries.iter().take(13).enumerate() {
        let rank = i + 1;
        let av = e.avatar_png.as_ref().map(|b| b64(b));
        if rank <= 3 {
            let c = &lay.podium[rank - 1];
            avatars.push_str(&avatar_svg(i, c, &av));
            // Pseudo + XP sous le cercle du podium.
            labels.push_str(&text(c.cx, c.cy + c.r + 30.0, "middle", 26.0, &e.name));
            labels.push_str(&text(
                c.cx,
                c.cy + c.r + 60.0,
                "middle",
                24.0,
                &format!("{} XP", fmt_xp(e.xp)),
            ));
        } else {
            let idx = rank - 4; // 0..9
            let left = idx < 5;
            let y = lay.row_ys[idx % 5];
            let (ax, nx, xx) = if left {
                (lay.l_avatar_x, lay.l_name_x, lay.l_xp_x)
            } else {
                (lay.r_avatar_x, lay.r_name_x, lay.r_xp_x)
            };
            let c = Circle { cx: ax, cy: y, r: lay.row_avatar_r };
            avatars.push_str(&avatar_svg(i, &c, &av));
            labels.push_str(&text(nx, y + 8.0, "start", 24.0, &e.name));
            labels.push_str(&text(xx, y + 8.0, "middle", 22.0, &fmt_xp(e.xp)));
        }
    }

    let template_img = format!(
        "<image x=\"0\" y=\"0\" width=\"{w}\" height=\"{h}\" href=\"data:image/png;base64,{template_b64}\"/>",
        w = lay.w,
        h = lay.h,
    );
    // Template transparent (trous aux cercles) -> avatars DERRIERE, template
    // par-dessus (encadre l'avatar), textes tout devant. Sinon (opaque) ->
    // template au fond, avatars puis textes dessus.
    let body = if template_on_top {
        format!("{avatars}{template_img}{labels}")
    } else {
        format!("{template_img}{avatars}{labels}")
    };
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">{body}</svg>",
        w = lay.w,
        h = lay.h,
    )
}

/// Rend le classement d'une catégorie en PNG. `None` si le template est absent
/// ou le rendu échoue (le caller retombe alors sur l'embed texte).
pub fn render_leaderboard(category: Category, entries: &[LbEntry]) -> Option<Vec<u8>> {
    let template = std::fs::read(templates_dir().join(category.file()))
        .map_err(|e| warn!(error = %e, file = category.file(), "Template classement introuvable"))
        .ok()?;
    // Detecte la transparence : si le template a un canal alpha (trous aux
    // cercles), on le dessine PAR-DESSUS les avatars pour l'effet "encastre".
    let on_top = image::load_from_memory(&template)
        .map(|img| img.color().has_alpha())
        .unwrap_or(false);
    let svg = build_svg(&b64(&template), entries, &layout_for(category), on_top);

    let opt = usvg::Options {
        fontdb: fontdb(),
        ..Default::default()
    };
    let tree = usvg::Tree::from_str(&svg, &opt)
        .map_err(|e| warn!(error = %e, "Echec parse SVG classement"))
        .ok()?;
    let size = tree.size().to_int_size();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size.width(), size.height())?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    pixmap.encode_png().ok()
}
