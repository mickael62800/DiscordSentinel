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
    /// Passif de classe (identifiant interne)
    pub passif_key: &'static str,
    /// Description du passif pour l'affichage
    pub passif_description: &'static str,
    /// Message de revelation en combat
    pub passif_reveal: &'static str,
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
    passif_key: "berserker",
    passif_description: "Berserker : +25% ATK quand HP < 30%",
    passif_reveal: "La rage envahit {joueur}... Son attaque explose ! C'est un BOURRIN !",
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
    passif_key: "esquive",
    passif_description: "Esquive : 15% de chance d'esquiver par round",
    passif_reveal: "{joueur} fait un pas de cote et esquive completement le coup ! C'est un AGILE !",
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
    passif_key: "vampirisme",
    passif_description: "Vampirisme : vole 10% des degats infliges en HP",
    passif_reveal: "{joueur} aspire l'energie de son adversaire ! C'est un FOURBE !",
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
    passif_key: "blindage",
    passif_description: "Blindage : -5 degats recus par round",
    passif_reveal: "Le coup rebondit sur {joueur} comme sur un mur ! C'est un TANK !",
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

/// Verifie si un nom de classe est valide.
#[allow(dead_code)]
pub fn is_valid_class(name: &str) -> bool {
    matches!(name, "bourrin" | "agile" | "fourbe" | "tank")
}
