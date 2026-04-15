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
    /// Le split /shop attaque vs /shop defense lit ce champ.
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
