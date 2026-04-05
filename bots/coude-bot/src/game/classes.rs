/// Statistiques d'une classe de joueur.
pub struct ClassStats {
    pub name: &'static str,
    pub emoji: &'static str,
    pub base_atk: i32,
    pub base_def: i32,
    pub atk_growth: i32,
    pub def_growth: i32,
    pub dodge_chance: f64,
    pub steal_bonus: f64,
    pub description: &'static str,
}

pub const CLASS_BOURRIN: ClassStats = ClassStats {
    name: "bourrin",
    emoji: "\u{1f4aa}",
    base_atk: 25,
    base_def: 8,
    atk_growth: 4,
    def_growth: 1,
    dodge_chance: 0.0,
    steal_bonus: 0.0,
    description: "Frappe fort mais encaisse mal",
};

pub const CLASS_AGILE: ClassStats = ClassStats {
    name: "agile",
    emoji: "\u{1f3c3}",
    base_atk: 12,
    base_def: 18,
    atk_growth: 2,
    def_growth: 3,
    dodge_chance: 0.15,
    steal_bonus: 0.0,
    description: "Esquive souvent mais frappe faible",
};

pub const CLASS_FOURBE: ClassStats = ClassStats {
    name: "fourbe",
    emoji: "\u{1f5e1}\u{fe0f}",
    base_atk: 18,
    base_def: 14,
    atk_growth: 3,
    def_growth: 2,
    dodge_chance: 0.0,
    steal_bonus: 0.20,
    description: "Manipule les regles",
};

pub const CLASS_TANK: ClassStats = ClassStats {
    name: "tank",
    emoji: "\u{1f6e1}\u{fe0f}",
    base_atk: 8,
    base_def: 25,
    atk_growth: 1,
    def_growth: 4,
    dodge_chance: 0.0,
    steal_bonus: 0.0,
    description: "Lent mais increvable",
};

#[allow(dead_code)]
pub const ALL_CLASSES: &[&ClassStats] = &[&CLASS_BOURRIN, &CLASS_AGILE, &CLASS_FOURBE, &CLASS_TANK];

pub fn get_class(name: &str) -> &'static ClassStats {
    match name {
        "bourrin" => &CLASS_BOURRIN,
        "agile" => &CLASS_AGILE,
        "fourbe" => &CLASS_FOURBE,
        "tank" => &CLASS_TANK,
        _ => &CLASS_BOURRIN,
    }
}
