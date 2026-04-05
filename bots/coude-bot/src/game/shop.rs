/// Definition d'un objet achetable en boutique.
#[allow(dead_code)]
pub struct ShopItem {
    pub key: &'static str,
    pub name: &'static str,
    pub emoji: &'static str,
    pub price: i64,
    pub description: &'static str,
}

pub const SHOP_ITEMS: &[ShopItem] = &[
    ShopItem {
        key: "explosion",
        name: "Explosion",
        emoji: "\u{1f4a3}",
        price: 200,
        description: "Les deux joueurs perdent toute la mise",
    },
    ShopItem {
        key: "inversion",
        name: "Inversion",
        emoji: "\u{1f504}",
        price: 500,
        description: "Echange tes coins avec ceux de l'adversaire",
    },
    ShopItem {
        key: "mindgame",
        name: "Mindgame",
        emoji: "\u{1f9e0}",
        price: 150,
        description: "Vois le roll de l'adversaire avant de jouer",
    },
    ShopItem {
        key: "rage",
        name: "Rage",
        emoji: "\u{1f621}",
        price: 100,
        description: "+50 attaque mais -50 defense",
    },
    ShopItem {
        key: "surprise",
        name: "Attaque surprise",
        emoji: "\u{1f4a8}",
        price: 300,
        description: "L'adversaire ne peut pas refuser",
    },
    ShopItem {
        key: "double_coup",
        name: "Double coup",
        emoji: "\u{270a}\u{270a}",
        price: 250,
        description: "Lance le de deux fois et garde le meilleur",
    },
    ShopItem {
        key: "coup_traitre",
        name: "Coup traitre",
        emoji: "\u{1f5e1}\u{fe0f}",
        price: 350,
        description: "Ignore le bonus de defense adverse",
    },
];

pub fn get_item(key: &str) -> Option<&'static ShopItem> {
    SHOP_ITEMS.iter().find(|i| i.key == key)
}
