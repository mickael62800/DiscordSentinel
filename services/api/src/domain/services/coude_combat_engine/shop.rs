/// Definition d'un objet achetable en boutique.
#[allow(dead_code)]
pub struct ShopItem {
    pub key: &'static str,
    pub name: &'static str,
    pub emoji: &'static str,
    pub price: i64,
    pub description: &'static str,
    /// Categorie d'affichage pour le shop. Valeurs :
    /// - `"attaque"` : items offensifs actives par l'attaquant pour
    ///   infliger plus de degats ou desavantager l'adversaire.
    /// - `"defense"` : items defensifs ou de soin (potions, bouclier,
    ///   antidote, explosion qui est une carte "defender only").
    /// - `"braquage"` : items consommables pour /braquage, chacun
    ///   apporte +5 % de chance au roll. Consommes tous a la tentative.
    /// Le split /shop attaque / defense / braquage lit ce champ.
    pub category: &'static str,
}

impl ShopItem {
    /// Helper pour le bot : filtrer les items d'une categorie.
    pub fn is_attaque(&self) -> bool {
        self.category == "attaque"
    }
    pub fn is_defense(&self) -> bool {
        self.category == "defense"
    }
}

pub const SHOP_ITEMS: &[ShopItem] = &[
    // ── Items d'attaque (offensifs) ──
    ShopItem {
        key: "rage",
        name: "Rage",
        emoji: "\u{1f621}",
        price: 100,
        description: "+50% ATK mais -30% DEF pendant le combat",
        category: "attaque",
    },
    ShopItem {
        key: "mindgame",
        name: "Mindgame",
        emoji: "\u{1f9e0}",
        price: 150,
        description: "Revele la classe et les HP de l'adversaire avant le combat",
        category: "attaque",
    },
    ShopItem {
        key: "double_coup",
        name: "Double Coup",
        emoji: "\u{270a}\u{270a}",
        price: 250,
        description: "Lance 2d20 et garde le meilleur a chaque round",
        category: "attaque",
    },
    ShopItem {
        key: "poison",
        name: "Poison",
        emoji: "\u{2620}\u{fe0f}",
        price: 300,
        description: "L'adversaire perd 5 HP par round pendant le combat",
        category: "attaque",
    },
    ShopItem {
        key: "surprise",
        name: "Attaque Surprise",
        emoji: "\u{1f4a8}",
        price: 300,
        description: "L'adversaire ne peut pas refuser le defi",
        category: "attaque",
    },
    ShopItem {
        key: "coup_traitre",
        name: "Coup Traitre",
        emoji: "\u{1f5e1}\u{fe0f}",
        price: 350,
        description: "Reduit la DEF adverse de 50% pendant le combat",
        category: "attaque",
    },
    // ── Items de defense (potions, boucliers, immunites, explosion) ──
    ShopItem {
        key: "potion_soin",
        name: "Potion de Soin",
        emoji: "\u{1f9ea}",
        price: 80,
        description: "+30 HP (utilisable hors combat)",
        category: "defense",
    },
    ShopItem {
        key: "antidote",
        name: "Antidote",
        emoji: "\u{1f49a}",
        price: 150,
        description: "Immunise contre le poison pendant 1 combat",
        category: "defense",
    },
    ShopItem {
        key: "potion_majeure",
        name: "Potion Majeure",
        emoji: "\u{1f48a}",
        price: 200,
        description: "+80 HP (utilisable hors combat)",
        category: "defense",
    },
    ShopItem {
        key: "explosion",
        name: "Explosion",
        emoji: "\u{1f4a3}",
        price: 200,
        description: "Annule le combat : les 2 joueurs perdent 50% de la mise (defenseur uniquement)",
        category: "defense",
    },
    ShopItem {
        key: "bouclier",
        name: "Bouclier",
        emoji: "\u{1f6e1}\u{fe0f}",
        price: 250,
        description: "+20% DEF pendant tout le combat",
        category: "defense",
    },
    // Phase 9 Part B : les items anti-vol historiques
    // (chien_garde / camera_surveillance / coffre_fort) ont ete migres
    // vers un modele d'abonnement temps-base dans `coude_steal_protection`.
    // Ils s'achetent desormais via `/protection` et sont invisibles
    // aux voleurs. Le shop ne les expose plus — un vieux stock eventuel
    // dans `coude_inventory` est mis a 0 par la migration 125.

    // ── Phase 10 : items de braquage (consommables) ──
    // Chacun apporte +5 % de chance au roll /braquage. Tous les items
    // presents dans l'inventaire sont consommes a la tentative (reussie
    // ou ratee). Source de verite : `domain::entities::coude_heist`.
    ShopItem {
        key: "masque_braquage",
        name: "Masque de braquage",
        emoji: "\u{1f3ad}",
        price: 100,
        description: "+5 % /braquage. Consomme a la tentative.",
        category: "braquage",
    },
    ShopItem {
        key: "pied_de_biche",
        name: "Pied-de-biche",
        emoji: "\u{1f528}",
        price: 150,
        description: "+5 % /braquage. Force les portes arriere.",
        category: "braquage",
    },
    ShopItem {
        key: "crochet_vault",
        name: "Crochet de vault",
        emoji: "\u{1f513}",
        price: 220,
        description: "+5 % /braquage. Plus discret que l'explosif.",
        category: "braquage",
    },
    ShopItem {
        key: "plan_coffre",
        name: "Plan du coffre",
        emoji: "\u{1f5fa}\u{fe0f}",
        price: 320,
        description: "+5 % /braquage. La moitie du boulot est deja fait.",
        category: "braquage",
    },
    ShopItem {
        key: "fumigene_diversion",
        name: "Fumigene de diversion",
        emoji: "\u{1f4a8}",
        price: 450,
        description: "+5 % /braquage. Sors discret.",
        category: "braquage",
    },
    ShopItem {
        key: "explosif",
        name: "Explosif",
        emoji: "\u{1f4a3}",
        price: 600,
        description: "+5 % /braquage. La methode directe.",
        category: "braquage",
    },
    ShopItem {
        key: "hacker_kit",
        name: "Hacker kit",
        emoji: "\u{1f4be}",
        price: 800,
        description: "+5 % /braquage. Bypass total des alarmes.",
        category: "braquage",
    },
    ShopItem {
        key: "drone_espion",
        name: "Drone espion",
        emoji: "\u{1f681}",
        price: 1000,
        description: "+5 % /braquage. Reperage aerien avant le coup.",
        category: "braquage",
    },
    ShopItem {
        key: "equipe_de_pros",
        name: "Equipe de pros",
        emoji: "\u{1f46a}",
        price: 1500,
        description: "+5 % /braquage. Tu n'es plus seul sur le coup.",
        category: "braquage",
    },
];

/// Liste des items anti-vol tries par efficacite decroissante.
///
/// DEPRECATED Phase 9 Part B : conserve vide pour retrocompat avec les
/// callers qui iter dessus. Toute la logique anti-vol passe maintenant
/// par le domain `coude_steal_protection` et le RPC
/// `TryTriggerStealProtection` cote API.
pub const ANTI_THEFT_ITEMS: &[(&str, u32)] = &[];

pub fn get_item(key: &str) -> Option<&'static ShopItem> {
    SHOP_ITEMS.iter().find(|i| i.key == key)
}

/// Verifie si un item est une potion (utilisable hors combat).
#[allow(dead_code)]
pub fn is_potion(key: &str) -> bool {
    matches!(key, "potion_soin" | "potion_majeure")
}

/// Retourne le montant de HP restaure par une potion.
#[allow(dead_code)]
pub fn potion_heal_amount(key: &str) -> i32 {
    match key {
        "potion_soin" => 30,
        "potion_majeure" => 80,
        _ => 0,
    }
}
