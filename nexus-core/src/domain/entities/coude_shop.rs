//! Catalogue pur des objets Coup de Coude.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShopItem {
    pub key: &'static str,
    pub name: &'static str,
    pub price: i64,
    pub description: &'static str,
}

pub const ITEMS: &[ShopItem] = &[
    ShopItem { key: "rage", name: "Rage", price: 100, description: "+50 ATK pour un combat" },
    ShopItem { key: "mindgame", name: "Mindgame", price: 150, description: "Revele un jet adverse" },
    ShopItem { key: "explosion", name: "Explosion", price: 200, description: "Annule le gain du duel" },
    ShopItem { key: "double_coup", name: "Double Coup", price: 250, description: "Garde le meilleur de deux jets" },
    ShopItem { key: "surprise", name: "Attaque Surprise", price: 300, description: "Defi immediat" },
    ShopItem { key: "coup_traitre", name: "Coup Traitre", price: 350, description: "Ignore la defense" },
    ShopItem { key: "inversion", name: "Inversion", price: 500, description: "Echange les soldes" },
];

pub fn item(key: &str) -> Option<ShopItem> { ITEMS.iter().copied().find(|item| item.key == key) }
