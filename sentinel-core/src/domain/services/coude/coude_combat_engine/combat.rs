use rand::Rng;

use super::chaos;
use super::chaos::ChaosEvent;
use super::classes;
use super::progression;
use super::PlayerLite as Player;
use super::ServerEventLite as ServerEvent;
use crate::domain::entities::coude::balance::BalanceParams;

// ══════════════════════════════════════════════════════════════════════
// ── Flavor text ──
// ══════════════════════════════════════════════════════════════════════

const COMBAT_START: &[&str] = &[
    "\u{2694}\u{fe0f} {attaquant} craque ses doigts et regarde {defenseur} droit dans les yeux...",
    "\u{1f514} DING DING ! Le match {attaquant} vs {defenseur} commence !",
    "\u{1f3ac} Les lumieres s'eteignent... le spot s'allume sur {attaquant} et {defenseur} !",
    "\u{1f32a}\u{fe0f} L'arene tremble ! {attaquant} et {defenseur} entrent en scene !",
    "\u{2620}\u{fe0f} Ca va saigner ! {attaquant} defie {defenseur} ! Prenez vos popcorns !",
    "\u{1f3b5} {attaquant} lance la musique de Rocky... {defenseur} sue deja !",
    "\u{1f409} {attaquant} active son Kaioken ! {defenseur} sent une aura hostile !",
    "\u{1f43c} {attaquant} vs {defenseur} : le Guerrier Dragon est en approche !",
    "\u{1f525} {attaquant} souffle sur ses poings, {defenseur} prie tous les dieux !",
    "\u{1f3a9} {attaquant} retire son chapeau, {defenseur} sait qu'il va souffrir !",
    "\u{1f9c3} {attaquant} pose sa biere et se leve... {defenseur} a reveille le fauve !",
    "\u{1f6a8} Alerte baston ! {attaquant} charge sur {defenseur} a pleine vitesse !",
    "\u{1f3af} {attaquant} a choisi sa cible : {defenseur}. Plus de retour en arriere !",
    "\u{1f4e2} Mesdames et messieurs, dans le coin rouge : {attaquant} ! Dans le coin bleu : {defenseur} !",
    "\u{1f37f} {attaquant} enleve sa veste, {defenseur} regrette deja d'etre ne !",
    "\u{26a1} {attaquant} charge son ultime, {defenseur} n'a pas backup ses saves !",
    "\u{1f60e} {attaquant} chausse ses lunettes de soleil et marche vers {defenseur}...",
    "\u{1f32a}\u{fe0f} Le ciel s'assombrit... {attaquant} est la pour en finir avec {defenseur} !",
    "\u{1f3b8} Solo de guitare ! {attaquant} debarque en mode rockstar sur {defenseur} !",
    "\u{1f984} {attaquant} invoque la magie du coup de coude contre {defenseur} !",
    "\u{1f477} {attaquant} sort la boite a outils. {defenseur} va etre demonte piece par piece !",
    "\u{1f431}\u{200d}\u{1f464} {attaquant} apparait dans l'ombre... {defenseur} n'a rien vu venir !",
    "\u{1f9d9} {attaquant} murmure un sortilege ; {defenseur} n'a meme pas appris a se defendre !",
    "\u{1f3ad} {attaquant} salue le public, {defenseur} l'ignore — grossiere erreur.",
    "\u{1f4ac} {attaquant} balance une derniere insulte a {defenseur} avant le premier coup !",
    "\u{1f4f0} BREAKING NEWS : {attaquant} arrive sur {defenseur} en direct du salon !",
    "\u{1f9ee} {attaquant} fait ses calculs sur les chances de {defenseur}... resultat : zero.",
    "\u{1f4dc} Le greffier ouvre le registre : combat n* 1042, {attaquant} vs {defenseur}.",
    "\u{1f3a8} {attaquant} signe l'arene de son blaze, {defenseur} y est juste de passage.",
    "\u{1f309} L'arc-en-ciel s'eteint : {attaquant} a decide de detruire {defenseur} aujourd'hui.",
    "\u{1f987} Comme une chauve-souris, {attaquant} surgit ! {defenseur} n'a pas le radar !",
    "\u{1f4cd} Cible verrouillee : {defenseur}. Tireur : {attaquant}. Issue : douloureuse.",
];

const ROUND_ATTACK: &[&str] = &[
    "\u{1f4a5} {attaquant} envoie un coup de coude VIOLENT ! {degats} degats !",
    "\u{1f44a} {attaquant} frappe avec precision ! {degats} degats !",
    "\u{1f9b5} {attaquant} enchaine avec un coup vicieux ! {degats} degats !",
    "\u{1f4ab} {attaquant} met toute sa force dans ce coup ! {degats} degats !",
    "\u{1f94a} BOUM ! {attaquant} connecte un coup solide ! {degats} degats !",
    "\u{1f528} {attaquant} sort le marteau imaginaire ! {degats} degats cranio-faciaux !",
    "\u{1f3af} Coup critique ! {attaquant} vise le point faible ! {degats} degats !",
    "\u{1f409} {attaquant} deploie son Kamehameha du coude ! {degats} degats !",
    "\u{26a1} Vitesse de la lumiere ! {attaquant} touche avant qu'on voie le mouvement ! {degats} degats !",
    "\u{1f980} {attaquant} pince le nerf principal ! {degats} degats d'agonie !",
    "\u{1f525} {attaquant} frappe avec la force de mille soleils ! {degats} degats !",
    "\u{1f3b8} Headbang musical ! {attaquant} scorche son adversaire ! {degats} degats !",
    "\u{1fa78} {attaquant} fait gicler du sang ! {degats} degats bien saignants !",
    "\u{1f525} REKT ! {attaquant} poutre son adversaire ! {degats} degats !",
    "\u{1f4a5} Coup combo x3 ! {attaquant} enchaine sans repit ! {degats} degats !",
    "\u{1f4a2} {attaquant} transforme son coude en enclume ! {degats} degats !",
    "\u{1f9ce}\u{200d}\u{2642}\u{fe0f} {attaquant} fait un uppercut de legende ! {degats} degats au menton !",
    "\u{1f343} {attaquant} frappe comme si sa vie en dependait ! {degats} degats !",
    "\u{1f3ae} Combo finisher ! {attaquant} deboite tout ! {degats} degats !",
    "\u{1f529} {attaquant} assene un coup de coude bien carre ! {degats} degats !",
    "\u{1f4a3} Explosion ! {attaquant} detone un coup atomique ! {degats} degats !",
    "\u{1f4aa} {attaquant} invoque la force de ses ancetres ! {degats} degats !",
    "\u{1f3af} Bullseye ! {attaquant} touche pile au bon endroit ! {degats} degats !",
    "\u{1f98d} KING KONG COUDE ! {attaquant} ecrase tout ! {degats} degats !",
    "\u{1f4a5} {attaquant} brise le mur du son ! {degats} degats supersoniques !",
    "\u{1f30b} Eruption ! {attaquant} libere sa lave interieure ! {degats} degats brulants !",
    "\u{1f30a} Vague de choc ! {attaquant} balaie tout sur son passage ! {degats} degats liquides !",
    "\u{2744}\u{fe0f} {attaquant} congele puis brise son adversaire ! {degats} degats glaces !",
    "\u{1f320} Pluie d'etoiles ! {attaquant} fait pleuvoir les coups ! {degats} degats stellaires !",
    "\u{1f680} {attaquant} decolle vers la lune et atterrit en plein dessus ! {degats} degats orbital !",
    "\u{1f3af} HEADSHOT ! {attaquant} touche le centre de gravite ! {degats} degats cibles !",
    "\u{1f9ec} {attaquant} reecrit l'ADN de son adversaire ! {degats} degats genetiques !",
    "\u{1f44a} {attaquant} sort le coup secret de papi ! {degats} degats inter-generationnels !",
    "\u{1f3ad} {attaquant} fait croire qu'il rate, puis touche pour {degats} degats ! Quel troll !",
    "\u{1f5e1}\u{fe0f} {attaquant} dechaine sa lame imaginaire ! {degats} degats mystiques !",
    "\u{1f4dd} Le coup est tellement bien que c'est ecrit dans les manuels ! {degats} degats academiques !",
];

const ROUND_WEAK: &[&str] = &[
    "\u{1f6e1}\u{fe0f} {defenseur} encaisse sans broncher ! {degats} degats seulement.",
    "\u{1f634} {attaquant} tape comme un chatonnet... {degats} degats.",
    "\u{1f9f1} {defenseur} est un MUR. {degats} petits degats.",
    "\u{1f41c} {attaquant} chatouille {defenseur}. {degats} degats.",
    "\u{1f98b} {attaquant} caresse {defenseur} avec un papillon. {degats} degats ridicules.",
    "\u{1f643} {attaquant} rate son coup... {defenseur} perd {degats} degats par pitie.",
    "\u{1f37d}\u{fe0f} {attaquant} utilise une cuillere en plastique. {degats} degats.",
    "\u{1f9fb} {defenseur} essuie le coup comme une miette. {degats} degats.",
    "\u{1f922} {attaquant} a saute le petit-dej, il tape mou. {degats} degats.",
    "\u{1f4a8} Coup de vent ! {attaquant} rate a moitie. {degats} degats.",
    "\u{1f95b} {defenseur} boit son lait tranquille. {degats} degats inoffensifs.",
    "\u{1f636} {attaquant} a oublie comment se battre ? {degats} degats genants.",
    "\u{1f480} {attaquant} cogne dans le vide, touche par ricochet. {degats} degats.",
    "\u{1f437} Groiiiink ! {defenseur} rigole du coup de {attaquant}. {degats} degats.",
    "\u{1f363} Sushi slap ! {attaquant} touche avec un concombre. {degats} degats.",
    "\u{1f9d3} {attaquant} tape comme un papi. {defenseur} prend {degats} degats.",
    "\u{1f411} {defenseur} prend {degats} degats de laine. Ca pique a peine.",
    "\u{1f32b}\u{fe0f} {attaquant} rate dans le brouillard ! {degats} degats accidentels.",
    "\u{1f3b2} Roll de degats catastrophique : {degats} pour {attaquant}...",
    "\u{1f47b} {attaquant} attaque dans le vide... un fantome encaisse a sa place. {degats} degats.",
    "\u{1f422} {attaquant} est aussi rapide qu'une tortue. {defenseur} prend {degats} degats en baillant.",
    "\u{1f4ac} {attaquant} bavarde au lieu de frapper. {degats} degats par accident.",
    "\u{1f9d3}\u{200d}\u{2640}\u{fe0f} Niveau papi : {attaquant} colle {degats} degats avec sa canne.",
    "\u{1f6cf}\u{fe0f} {attaquant} pousse une couette sur {defenseur}. {degats} degats moelleux.",
    "\u{1f344} {attaquant} jette un champignon... ca aurait pu etre pire. {degats} degats.",
    "\u{1f9c1} Coup sucre ! {attaquant} balance un cupcake ! {degats} degats sucres.",
    "\u{1f43b} {attaquant} fait un calin a {defenseur}, c'est embarrassant. {degats} degats sociaux.",
    "\u{1f486} Massage offensif ! {attaquant} relaxe douloureusement {defenseur}. {degats} degats.",
    "\u{1f5d2}\u{fe0f} Lance-papier ! {attaquant} balance des post-its ! {degats} degats administratifs.",
    "\u{1f3a8} {attaquant} bombarde {defenseur} de peinture aquarelle. {degats} degats artistiques.",
];

const COMBAT_KO: &[&str] = &[
    "\u{2620}\u{fe0f} {perdant} s'ecroule ! K.O. ! {gagnant} remporte le combat !",
    "\u{1f480} C'est TERMINE ! {perdant} est a terre ! {gagnant} leve le poing !",
    "\u{1f3c6} {gagnant} acheve {perdant} avec un dernier coup ! VICTOIRE !",
    "\u{1faa6} Repose en paix la dignite de {perdant}. {gagnant} domine !",
    "\u{1f4a4} {perdant} visite le royaume des reves. {gagnant} empoche la gloire !",
    "\u{1f6cc} {perdant} fait dodo pour la journee. {gagnant} s'en va tranquille !",
    "\u{1f90f} {gagnant} GG EZ ! {perdant} retourne au menu principal !",
    "\u{1f52b} Finish him ! {gagnant} deboite {perdant} sans pitie !",
    "\u{1f3ae} Game over pour {perdant} ! {gagnant} insere une nouvelle piece !",
    "\u{1fabd} {perdant} compte les etoiles au plafond. {gagnant} est deja parti faire la fete !",
    "\u{1f9fc} {gagnant} nettoie le tapis ou gisait {perdant}. Travail acheve !",
    "\u{1f3c1} Drapeau a damier ! {gagnant} remporte la course, {perdant} est reste au stand !",
    "\u{1f9e8} {perdant} explose en feu d'artifice ! {gagnant} applaudit !",
    "\u{1f48a} Le medecin a besoin de voir {perdant} maintenant. {gagnant} rentre invaincu !",
    "\u{1f3f4} Drapeau blanc ! {perdant} ne reviendra pas. {gagnant} regne !",
    "\u{1f514} {gagnant} sonne la cloche, {perdant} ne se relevera pas avant demain !",
    "\u{1f4de} Allo ambulance ? C'est pour {perdant}. {gagnant} vous salue bien !",
    "\u{1f947} {gagnant} monte sur le podium, {perdant} descend aux urgences !",
    "\u{1f3a4} {gagnant} lache le mic ! {perdant} est au tapis, KO technique !",
    "\u{1f3ad} Rideau ! Le spectacle de {perdant} se termine en larmes. {gagnant} salue !",
    "\u{1f5fd} {gagnant} plante son drapeau dans le crane de {perdant}. Domination totale !",
    "\u{1faa6} {perdant} part rejoindre les anciens combattants. {gagnant} reste seul au sommet.",
    "\u{1f396}\u{fe0f} Medaille d'or pour {gagnant} ! {perdant} repart avec une compresse.",
    "\u{1fa78} Hemoglobine partout ! {perdant} dort dans une mare. {gagnant} marche vers la lumiere !",
    "\u{1f5dd}\u{fe0f} Game over, insert coin... ah non, {perdant} n'a plus de credit. {gagnant} continue !",
    "\u{1f3aa} The show is over ! {gagnant} gagne le grand prix, {perdant} remballe le clown.",
    "\u{2728} {gagnant} brille de mille feux. {perdant} ne brille plus du tout, sniff.",
    "\u{1f4ff} Final fatality ! {gagnant} EXTERMINE {perdant} pour de bon !",
    "\u{1f9ff} {gagnant} place une amulette sur {perdant} pour eviter qu'il revienne hanter !",
];

const COMBAT_TIMEOUT: &[&str] = &[
    "\u{23f0} TEMPS ECOULE ! {gagnant} gagne aux points ({hp_g}% HP vs {hp_p}% HP) !",
    "\u{1f514} Fin du match ! {gagnant} l'emporte avec {hp_g}% de vie restante !",
    "\u{1f4ca} Les juges tranchent : {gagnant} gagne avec {hp_g}% HP contre {hp_p}% !",
    "\u{1f4dc} Verdict du jury : {gagnant} victorieux ! {hp_g}% HP contre {hp_p}% HP !",
    "\u{1f4f8} Photo finish ! {gagnant} l'emporte de justesse, {hp_g}% vs {hp_p}% !",
    "\u{1f3b2} Ding ! {gagnant} remporte le combat aux points, {hp_g}% contre {hp_p}% !",
    "\u{1f9ee} Calculs faits : {gagnant} gagne avec {hp_g}% HP. Adversaire a {hp_p}%.",
    "\u{1f4c8} Stats finales : {gagnant} sort vainqueur, {hp_g}% vs {hp_p}% HP !",
    "\u{1f3a3} La cloche sonne ! {gagnant} attrape la victoire, {hp_g}% HP vs {hp_p}% !",
    "\u{1f3c5} Decision partagee ? Que nenni ! {gagnant} gagne, {hp_g}% vs {hp_p}% !",
    "\u{1f9d1}\u{200d}\u{2696}\u{fe0f} Sur le fil ! {gagnant} remporte le combat, {hp_g}% contre {hp_p}% !",
    "\u{26f3} {gagnant} gagne aux points avec {hp_g}% HP (adversaire : {hp_p}%) !",
    "\u{1f5fd} {gagnant} tient la barre ! Victoire aux points, {hp_g}% vs {hp_p}% !",
    "\u{1f3af} Cible atteinte ! {gagnant} sort gagnant ({hp_g}% HP contre {hp_p}%) !",
    "\u{2696}\u{fe0f} La balance penche pour {gagnant} : {hp_g}% HP face a {hp_p}% pour l'autre.",
    "\u{1f4dc} Le verdict du parchemin sacre : {gagnant} l'emporte avec {hp_g}% HP !",
    "\u{1f5e3}\u{fe0f} {gagnant} tient debout, l'autre boite. Score final : {hp_g}% vs {hp_p}% !",
    "\u{1f4f6} Couverture reseau plus stable que {hp_p}%. {gagnant} l'emporte avec {hp_g}% HP !",
    "\u{1f3aa} La foule clame : {gagnant} ! Il termine a {hp_g}% HP, {hp_p}% pour l'autre.",
    "\u{1f4f8} Photo selfie post-victoire : {gagnant} a {hp_g}% HP encore en stock.",
    "\u{1f9d1}\u{200d}\u{2696}\u{fe0f} Les juges chuchotent, puis tranchent : {gagnant} gagne, {hp_g}% vs {hp_p}%.",
    "\u{1f9ed} La boussole pointe vers {gagnant} (HP {hp_g}% restants, {hp_p}% pour le perdant).",
    "\u{1f527} {gagnant} repare l'arene avec ses {hp_g}% HP restants. L'autre n'en a plus que {hp_p}%.",
];

const COMBAT_DRAW: &[&str] = &[
    "\u{1f91d} Les deux combattants sont a bout de souffle ! Match nul !",
    "\u{2696}\u{fe0f} Impossible de les departager ! Egalite parfaite !",
    "\u{1fae0} Personne ne gagne... personne ne perd... c'est frustrant.",
    "\u{1f92f} Les deux sont au tapis en meme temps ! Double KO !",
    "\u{1f3b2} Le destin a tranche : match nul, reessayez plus tard !",
    "\u{1f37b} Bon, on va boire un coup ? Combat indecidable !",
    "\u{1f6d1} Stop ! Les deux cotes sont epuises ! Draw !",
    "\u{1fa84} La magie du combat a fait une egalite ! Personne ne gagne !",
    "\u{1f914} Meme les juges sont confus... match nul !",
    "\u{1f937} Bon bah... c'est nul des deux cotes ! Aucun vainqueur !",
    "\u{1f3ad} Rideau gele : les deux comediens oublient leur texte. Match nul !",
    "\u{1f9ff} Les esprits du combat boudent, refusent de trancher. Egalite !",
    "\u{1f6f8} Atterrissage des aliens : ils interrompent. Combat indecidable !",
    "\u{1f4ad} Les deux pensent a leur maman en meme temps. Trop attendrissant pour finir.",
    "\u{1f955} Les deux mangent une carotte cosmique et oublient pourquoi ils se tapaient. Egalite !",
    "\u{1f3a8} Les deux dessinent un coeur dans le sable. Match nul artistique !",
    "\u{1f6cf}\u{fe0f} Tout le monde au lit, demain on recommence. Match nul fatigue !",
    "\u{1f3b6} Une symphonie surgit, les deux dansent ensemble. Personne ne se bat, fin !",
    "\u{1f5ff} Les deux se transforment en statues. Ca va etre dur de finir le combat...",
    "\u{1f37e} Bouchon de champagne ! Les deux trinquent. Match nul, on s'en fout !",
];

// ══════════════════════════════════════════════════════════════════════
// ── Structs ──
// ══════════════════════════════════════════════════════════════════════

/// Result of a single round.
#[allow(dead_code)]
pub struct RoundResult {
    pub round_number: i32,
    pub attacker_roll: i32,
    pub defender_roll: i32,
    pub attacker_damage: i32,
    pub defender_damage: i32,
    pub attacker_hp_after: i32,
    pub defender_hp_after: i32,
    pub chaos_event: Option<ChaosEvent>,
    pub attacker_passif: Option<String>,
    pub defender_passif: Option<String>,
    pub message: String,
}

/// Full combat result.
#[allow(dead_code)]
pub struct CombatResult {
    pub winner_id: Option<String>,
    pub loser_id: Option<String>,
    pub rounds: Vec<RoundResult>,
    pub total_rounds: i32,
    pub attacker_hp_final: i32,
    pub defender_hp_final: i32,
    pub attacker_hp_max: i32,
    pub defender_hp_max: i32,
    pub chaos_events_count: i32,
    pub coins_won: i64,
    pub coins_lost_by_loser: i64,
    pub stolen_bonus: i64,
    pub vol_coins: i64,
    pub message: String,
    pub is_giant_killer: bool,
    pub attacker_class_revealed: Option<String>,
    pub defender_class_revealed: Option<String>,
}

// ══════════════════════════════════════════════════════════════════════
// ── Helpers ──
// ══════════════════════════════════════════════════════════════════════

fn pick_random<'a>(templates: &'a [&'a str]) -> &'a str {
    let mut rng = rand::thread_rng();
    let idx = rng.gen_range(0..templates.len());
    templates[idx]
}

fn fmt_template(template: &str, replacements: &[(&str, &str)]) -> String {
    let mut s = template.to_string();
    for (key, val) in replacements {
        s = s.replace(key, val);
    }
    s
}

/// Calcule les stats effectives d'un joueur (ATK, DEF).
pub fn effective_stats(player: &Player) -> (i32, i32) {
    let class = classes::get_class(player.class.as_deref().unwrap_or("bourrin"));
    let atk = class.base_atk + (player.level - 1) * class.atk_growth + player.atk;
    let def = class.base_def + (player.level - 1) * class.def_growth + player.def;
    (atk, def)
}

/// Calculate maximum HP for a player.
pub fn calculate_hp_max(player: &Player) -> i32 {
    let (_, def) = effective_stats(player);
    100 + def * 2
}

/// Calculate damage for one hit.
fn calc_damage(roll: i32, atk: i32, enemy_def: i32) -> i32 {
    let degats_bruts = (roll as f64 * atk as f64) / 10.0;
    let reduction = enemy_def as f64 / (enemy_def as f64 + 50.0);
    let degats = degats_bruts * (1.0 - reduction);
    3i32.max(degats as i32)
}

/// Determine max rounds from combined HP.
fn max_rounds(combined_hp: i32) -> i32 {
    if combined_hp < 250 {
        3
    } else if combined_hp <= 400 {
        5
    } else {
        7
    }
}

// ══════════════════════════════════════════════════════════════════════
// ── Main combat function ──
// ══════════════════════════════════════════════════════════════════════

/// Maledictions actives sur les combattants (cf. COUPE_AMELIORATIONS 5.1)
/// + modificateurs de saison thematique (cf. 6.3). Default = aucune
/// malediction et pas de modulation de saison.
#[derive(Debug, Default, Clone, Copy)]
pub struct CombatCurses {
    pub attacker_has_banana: bool,
    pub defender_has_banana: bool,
    /// Multiplicateur de probabilite des chaos events. None = neutre
    /// (1.0). Some(2.0) sous "Saison du Chaos".
    pub chaos_multiplier: Option<f64>,
    /// Bonus DEF (en %) applique aux Tanks uniquement, avant items
    /// (cf. COUPE_AMELIORATIONS 6.3 Saison du Tank). None = neutre.
    pub tank_def_bonus_pct: Option<f64>,
    /// Cf. COUPE_AMELIORATIONS 3.2 palier "Riposte fulgurante" (niveau 20+
    /// vs joueur < lv). Si `true`, au round 1 les degats du defenseur
    /// sont appliques AVANT ceux de l attaquant ; si l attaquant meurt
    /// avant d avoir pu frapper, ses degats sont annules.
    pub defender_riposte_first_round: bool,
    /// Probabilite (0..1) qu une ligne de flavor soit injectee par round.
    /// None = defaut historique (`FLAVOR_LINE_PROBABILITY` = 0.20). Reglable
    /// par serveur via `CoudeEconomyConfig::flavor_line_probability`.
    pub flavor_line_probability: Option<f64>,
}

pub fn resolve_combat(
    attacker: &Player,
    defender: &Player,
    attacker_current_hp: i32,
    defender_current_hp: i32,
    mise: i64,
    special: Option<&str>,
    defender_special: Option<&str>,
    active_events: &[ServerEvent],
    params: &BalanceParams,
) -> CombatResult {
    resolve_combat_with_curses(
        attacker,
        defender,
        attacker_current_hp,
        defender_current_hp,
        mise,
        special,
        defender_special,
        active_events,
        params,
        CombatCurses::default(),
    )
}

pub fn resolve_combat_with_curses(
    attacker: &Player,
    defender: &Player,
    attacker_current_hp: i32,
    defender_current_hp: i32,
    mise: i64,
    special: Option<&str>,
    defender_special: Option<&str>,
    active_events: &[ServerEvent],
    params: &BalanceParams,
    curses: CombatCurses,
) -> CombatResult {
    // Pre-calcul des coefficients a partir des % config.
    let rage_atk_mult = 1.0 + (params.rage_atk_bonus_pct as f64 / 100.0);
    let rage_def_mult = 1.0 - (params.rage_def_malus_pct as f64 / 100.0);
    let coup_traitre_mult = 1.0 - (params.coup_traitre_def_malus_pct as f64 / 100.0);
    let bouclier_mult = 1.0 + (params.bouclier_def_bonus_pct as f64 / 100.0);
    let poison_dmg = params.poison_damage_per_round as i32;
    let mut rng = rand::thread_rng();

    let atk_class = classes::get_class(attacker.class.as_deref().unwrap_or("bourrin"));
    let def_class = classes::get_class(defender.class.as_deref().unwrap_or("bourrin"));

    // Base effective stats (classe + niveau + points). Sert aussi de base pour
    // calculer le hp_max du combat : on snapshot la DEF AVANT d'appliquer les
    // items (Rage, Coup Traitre, Bouclier) qui modifient atk_def/def_def. Sinon
    // un joueur fraichement /repos voit son HP current cape a tort par un
    // hp_max reduit (bug : « /repos pas pris en compte »).
    let (mut atk_atk, mut atk_def) = effective_stats(attacker);
    let (mut def_atk, mut def_def) = effective_stats(defender);

    // Saison du Tank (cf. COUPE_AMELIORATIONS 6.3) : +X% DEF pour les
    // Tanks uniquement, applique AVANT le snapshot hp_max pour que le
    // bonus se reflete aussi dans le HP pool.
    if let Some(pct) = curses.tank_def_bonus_pct {
        let mult = 1.0 + (pct / 100.0);
        if atk_class.name == "tank" {
            atk_def = (atk_def as f64 * mult) as i32;
        }
        if def_class.name == "tank" {
            def_def = (def_def as f64 * mult) as i32;
        }
    }

    let base_atk_def = atk_def;
    let base_def_def = def_def;

    // Matchmaking handicap
    let (handicap, _blocked) = progression::matchmaking_handicap(attacker.level, defender.level);
    let level_gap = (attacker.level - defender.level).abs();
    let stronger_is_attacker = attacker.level > defender.level;
    let stronger_is_defender = defender.level > attacker.level;

    if stronger_is_attacker && level_gap >= 3 {
        atk_atk = (atk_atk as f64 * handicap) as i32;
    }
    if stronger_is_defender && level_gap >= 3 {
        def_atk = (def_atk as f64 * handicap) as i32;
    }

    // ── Item effects (global, applied once) ──

    // Rage: +X% ATK, -Y% DEF (config)
    if special == Some("rage") {
        atk_atk = (atk_atk as f64 * rage_atk_mult) as i32;
        atk_def = (atk_def as f64 * rage_def_mult) as i32;
    }
    if defender_special == Some("rage") {
        def_atk = (def_atk as f64 * rage_atk_mult) as i32;
        def_def = (def_def as f64 * rage_def_mult) as i32;
    }

    // Coup traitre: -X% DEF adverse (config)
    if special == Some("coup_traitre") {
        def_def = (def_def as f64 * coup_traitre_mult) as i32;
    }
    if defender_special == Some("coup_traitre") {
        atk_def = (atk_def as f64 * coup_traitre_mult) as i32;
    }

    // Bouclier: +X% DEF (config)
    if special == Some("bouclier") {
        atk_def = (atk_def as f64 * bouclier_mult) as i32;
    }
    if defender_special == Some("bouclier") {
        def_def = (def_def as f64 * bouclier_mult) as i32;
    }

    // HP max = base DEF * 2 + 100 (PAS la DEF modifiee par les items). Les
    // buffs/debuffs DEF d'items n'ont d'effet que sur la reduction de degats
    // round par round, jamais sur le HP pool de depart. Sans ce snapshot, un
    // Tank frais apres /repos avec Rage/Coup Traitre perd silencieusement
    // des HP avant meme le round 1.
    let atk_hp_max = 100 + base_atk_def * 2;
    let def_hp_max = 100 + base_def_def * 2;

    let mut atk_hp = attacker_current_hp.min(atk_hp_max);
    let mut def_hp = defender_current_hp.min(def_hp_max);

    // Has double_coup?
    let atk_double = special == Some("double_coup");
    let def_double = defender_special == Some("double_coup");

    // Has poison?
    let atk_poison = special == Some("poison");
    let def_poison = defender_special == Some("poison");

    // Happy hour
    let happy_hour = active_events.iter().any(|e| e.event_type == "happy_hour");
    let multiplier = if happy_hour { 2 } else { 1 };

    // Cowardice penalty
    let coward_penalty_atk = if attacker.cowardice_count >= 5 {
        0.80
    } else {
        1.0
    };
    let coward_penalty_def = if defender.cowardice_count >= 5 {
        0.80
    } else {
        1.0
    };

    let atk_name = format!("<@{}>", attacker.user_id);
    let def_name = format!("<@{}>", defender.user_id);

    // ── Explosion: early exit, both lose 50% of mise ──
    if defender_special == Some("explosion") {
        let lost = (mise as f64 * 0.5) as i64;
        return CombatResult {
            winner_id: None,
            loser_id: None,
            rounds: vec![],
            total_rounds: 0,
            attacker_hp_final: atk_hp,
            defender_hp_final: def_hp,
            attacker_hp_max: atk_hp_max,
            defender_hp_max: def_hp_max,
            chaos_events_count: 0,
            coins_won: 0,
            coins_lost_by_loser: lost,
            stolen_bonus: 0,
            vol_coins: 0,
            message: format!(
                "\u{1f4a3} **EXPLOSION !** {} active une bombe ! Les deux perdent **{} coins** !",
                def_name, lost
            ),
            is_giant_killer: false,
            attacker_class_revealed: None,
            defender_class_revealed: None,
        };
    }

    // ── Combat start message ──
    let start_msg = fmt_template(
        pick_random(COMBAT_START),
        &[("{attaquant}", &atk_name), ("{defenseur}", &def_name)],
    );

    let rounds_max = max_rounds(atk_hp_max + def_hp_max);
    let mut rounds: Vec<RoundResult> = Vec::new();
    let mut chaos_count = 0;
    let mut vol_coins_total: i64 = 0;
    let mut attacker_class_revealed: Option<String> = None;
    let mut defender_class_revealed: Option<String> = None;

    // ══════════════════════════════════════════════════════════════════
    // ── Combat loop ──
    // ══════════════════════════════════════════════════════════════════

    for round_num in 1..=rounds_max {
        let mut round_msg = format!("**--- Round {} ---**\n", round_num);
        let mut atk_passif: Option<String> = None;
        let mut def_passif: Option<String> = None;

        // ── Rolls (avec branchement Banana cf. COUPE_AMELIORATIONS 5.1) ──
        // Si le combattant est sous Peau de banane, 30% de chance que son
        // d20 soit ramene a 1 (echec critique). Le tirage de probabilite
        // est independant du d20 lui-meme.
        let roll_d20 = |rng: &mut rand::rngs::ThreadRng, banana: bool| -> i32 {
            let raw: i32 = rng.gen_range(1..=20);
            if banana {
                use crate::domain::entities::coude::curse::apply_banana_to_d20;
                let p: f64 = rng.gen_range(0.0..1.0);
                apply_banana_to_d20(raw as u8, true, p) as i32
            } else {
                raw
            }
        };
        let mut atk_roll: i32 = roll_d20(&mut rng, curses.attacker_has_banana);
        let mut def_roll: i32 = roll_d20(&mut rng, curses.defender_has_banana);

        if atk_double {
            let second: i32 = roll_d20(&mut rng, curses.attacker_has_banana);
            atk_roll = params.double_coup_mode.aggregate(atk_roll, second);
        }
        if def_double {
            let second: i32 = roll_d20(&mut rng, curses.defender_has_banana);
            def_roll = params.double_coup_mode.aggregate(def_roll, second);
        }

        // ── Effective ATK this round (class passives) ──
        let mut atk_atk_round = atk_atk;
        let mut def_atk_round = def_atk;

        // Bourrin: Berserker — ATK +25% when HP <= 30%
        // (inclusif pour eviter l'off-by-one : a exactement 30% le passif
        // s'active, coherent avec le 50% / 25% inclusifs des autres seuils).
        let atk_berserker_threshold = (atk_hp_max as f64 * 0.3).ceil() as i32;
        let def_berserker_threshold = (def_hp_max as f64 * 0.3).ceil() as i32;
        if atk_class.name == "bourrin" && atk_hp <= atk_berserker_threshold {
            atk_atk_round = (atk_atk_round as f64 * 1.25) as i32;
            atk_passif = Some("berserker".to_string());
            attacker_class_revealed = Some("bourrin".to_string());
        }
        if def_class.name == "bourrin" && def_hp <= def_berserker_threshold {
            def_atk_round = (def_atk_round as f64 * 1.25) as i32;
            def_passif = Some("berserker".to_string());
            defender_class_revealed = Some("bourrin".to_string());
        }

        // ── Base damage calc ──
        let mut atk_dmg = calc_damage(atk_roll, atk_atk_round, def_def);
        let mut def_dmg = calc_damage(def_roll, def_atk_round, atk_def);

        // ── Tank: Blindage — reduce damage taken by 5 flat (after formula) ──
        // Exception : Tank vs Tank → les deux blindages s'annulent sinon on se
        // retrouve avec 1 dmg/round chacun et un timeout garanti (draw/accident).
        let tank_mirror = atk_class.name == "tank" && def_class.name == "tank";
        if !tank_mirror {
            if atk_class.name == "tank" {
                def_dmg = (def_dmg - 5).max(1);
                if atk_passif.is_none() {
                    atk_passif = Some("blindage".to_string());
                }
                attacker_class_revealed = Some("tank".to_string());
            }
            if def_class.name == "tank" {
                atk_dmg = (atk_dmg - 5).max(1);
                if def_passif.is_none() {
                    def_passif = Some("blindage".to_string());
                }
                defender_class_revealed = Some("tank".to_string());
            }
        } else {
            // Mirror match : on revele quand meme les classes pour la tension
            // mais aucun passif ne s'applique.
            attacker_class_revealed = Some("tank".to_string());
            defender_class_revealed = Some("tank".to_string());
            if atk_passif.is_none() {
                atk_passif = Some("tank_mirror".to_string());
            }
            if def_passif.is_none() {
                def_passif = Some("tank_mirror".to_string());
            }
        }

        // ── Agile: Esquive — dodge chance per round ──
        let atk_dodged = if atk_class.dodge_chance > 0.0 {
            rng.gen_bool(atk_class.dodge_chance.min(1.0))
        } else {
            false
        };
        let def_dodged = if def_class.dodge_chance > 0.0 {
            rng.gen_bool(def_class.dodge_chance.min(1.0))
        } else {
            false
        };

        if atk_dodged {
            def_dmg = 0;
            atk_passif = Some("esquive".to_string());
            attacker_class_revealed = Some("agile".to_string());
            round_msg.push_str(&format!("\u{1f3c3} {} esquive le coup !\n", atk_name));
        }
        if def_dodged {
            atk_dmg = 0;
            def_passif = Some("esquive".to_string());
            defender_class_revealed = Some("agile".to_string());
            round_msg.push_str(&format!("\u{1f3c3} {} esquive le coup !\n", def_name));
        }

        // ── Chaos event (8% per round) ──
        let chaos_event = chaos::roll_chaos_with_multiplier(curses.chaos_multiplier.unwrap_or(1.0));
        // We use roll_chaos which has 18% total; for now we treat it as-is
        // (will be adjusted to 8% per-round in chaos.rs separately)

        if let Some(ref ce) = chaos_event {
            chaos_count += 1;
            match ce {
                ChaosEvent::CritiqueSauvage => {
                    // x2 damage for whoever deals more this round
                    if atk_dmg >= def_dmg {
                        atk_dmg *= 2;
                        round_msg.push_str(&format!(
                            "{} **{}** — {} inflige x2 degats ce round !\n",
                            ce.emoji(),
                            ce.label(),
                            atk_name
                        ));
                    } else {
                        def_dmg *= 2;
                        round_msg.push_str(&format!(
                            "{} **{}** — {} inflige x2 degats ce round !\n",
                            ce.emoji(),
                            ce.label(),
                            def_name
                        ));
                    }
                }
                ChaosEvent::EsquiveDivine => {
                    // Defender dodges and counter-attacks with +50% damage
                    atk_dmg = 0;
                    def_dmg = (def_dmg as f64 * 1.5) as i32;
                    round_msg.push_str(&format!(
                        "{} **{}** — {} esquive et contre-attaque a +50% !\n",
                        ce.emoji(),
                        ce.label(),
                        def_name
                    ));
                }
                ChaosEvent::AccidentDebile => {
                    // Both take 10% of their max HP
                    let atk_self_dmg = (atk_hp_max as f64 * 0.1) as i32;
                    let def_self_dmg = (def_hp_max as f64 * 0.1) as i32;
                    atk_hp -= atk_self_dmg;
                    def_hp -= def_self_dmg;
                    round_msg.push_str(&format!(
                        "{} **{}** — Les deux prennent des degats ! ({} et {} HP perdus)\n",
                        ce.emoji(),
                        ce.label(),
                        atk_self_dmg,
                        def_self_dmg
                    ));
                }
                ChaosEvent::Glissade => {
                    // Attacker hits himself
                    atk_hp -= atk_dmg;
                    atk_dmg = 0;
                    round_msg.push_str(&format!(
                        "{} **{}** — {} se frappe lui-meme !\n",
                        ce.emoji(),
                        ce.label(),
                        atk_name
                    ));
                }
                ChaosEvent::Vol => {
                    // Winner of this round steals 5% of opponent's coins
                    let steal_amount = (mise as f64 * 0.05) as i64;
                    vol_coins_total += steal_amount;
                    round_msg.push_str(&format!(
                        "{} **{}** — {} coins voles en bonus !\n",
                        ce.emoji(),
                        ce.label(),
                        steal_amount
                    ));
                }
            }
        }

        // ── Apply poison (damage configurable) ──
        if atk_poison {
            def_hp -= poison_dmg;
            round_msg.push_str(&format!(
                "\u{2620}\u{fe0f} {} subit {} degats de poison !\n",
                def_name, poison_dmg
            ));
        }
        if def_poison {
            atk_hp -= poison_dmg;
            round_msg.push_str(&format!(
                "\u{2620}\u{fe0f} {} subit {} degats de poison !\n",
                atk_name, poison_dmg
            ));
        }

        // ── Apply damage ──
        // En regle generale c est simultane. Exception : palier "Riposte
        // fulgurante" (cf. COUPE_AMELIORATIONS 3.2) — au round 1, le
        // defenseur frappe en premier et si l attaquant meurt avant
        // d avoir pu placer son coup, ses degats sont annules.
        if curses.defender_riposte_first_round && round_num == 1 {
            atk_hp -= def_dmg;
            if atk_hp <= 0 {
                round_msg.push_str(&format!(
                    "\u{26a1} **Riposte fulgurante** : {} frappe en premier et abat {} avant qu il ne puisse riposter !\n",
                    def_name, atk_name
                ));
                atk_dmg = 0;
            }
            def_hp -= atk_dmg;
        } else {
            def_hp -= atk_dmg;
            atk_hp -= def_dmg;
        }

        // ── Fourbe: Vampirisme — heal 10% of damage dealt ──
        if atk_class.name == "fourbe" && atk_dmg > 0 {
            let heal = (atk_dmg as f64 * 0.1) as i32;
            atk_hp = (atk_hp + heal).min(atk_hp_max);
            if atk_passif.is_none() {
                atk_passif = Some("vampirisme".to_string());
            }
            attacker_class_revealed = Some("fourbe".to_string());
            if heal > 0 {
                round_msg.push_str(&format!(
                    "\u{1fa78} {} se soigne de {} HP (vampirisme) !\n",
                    atk_name, heal
                ));
            }
        }
        if def_class.name == "fourbe" && def_dmg > 0 {
            let heal = (def_dmg as f64 * 0.1) as i32;
            def_hp = (def_hp + heal).min(def_hp_max);
            if def_passif.is_none() {
                def_passif = Some("vampirisme".to_string());
            }
            defender_class_revealed = Some("fourbe".to_string());
            if heal > 0 {
                round_msg.push_str(&format!(
                    "\u{1fa78} {} se soigne de {} HP (vampirisme) !\n",
                    def_name, heal
                ));
            }
        }

        // Clamp HP to 0 minimum
        atk_hp = atk_hp.max(0);
        def_hp = def_hp.max(0);

        // ── Round flavor text ──
        if atk_dmg > 0 {
            let templates = if atk_dmg < 5 {
                ROUND_WEAK
            } else {
                ROUND_ATTACK
            };
            let txt = fmt_template(
                pick_random(templates),
                &[
                    ("{attaquant}", &atk_name),
                    ("{defenseur}", &def_name),
                    ("{degats}", &atk_dmg.to_string()),
                ],
            );
            round_msg.push_str(&txt);
            round_msg.push('\n');
        }
        if def_dmg > 0 {
            let templates = if def_dmg < 5 {
                ROUND_WEAK
            } else {
                ROUND_ATTACK
            };
            let txt = fmt_template(
                pick_random(templates),
                &[
                    ("{attaquant}", &def_name),
                    ("{defenseur}", &atk_name),
                    ("{degats}", &def_dmg.to_string()),
                ],
            );
            round_msg.push_str(&txt);
            round_msg.push('\n');
        }

        // Commentaires de combat debiles (cf. COUPE_AMELIORATIONS 2.2) —
        // ~20% par round, aucune incidence mecanique, juste de l ambiance.
        {
            use crate::domain::entities::coude::combat::flavor::pick_flavor_line;
            use crate::domain::entities::coude::combat::flavor::FLAVOR_LINE_PROBABILITY;
            let proba: f64 = rng.gen_range(0.0..1.0);
            let flavor_threshold = curses
                .flavor_line_probability
                .unwrap_or(FLAVOR_LINE_PROBABILITY);
            if let Some(line) =
                pick_flavor_line(&mut rng, proba, flavor_threshold, &atk_name, &def_name)
            {
                round_msg.push_str(&format!("\n_\u{1f3ad} {}_\n", line));
            }
        }

        round_msg.push_str(&format!(
            "\u{2764}\u{fe0f} {} : {}/{} HP | {} : {}/{} HP",
            atk_name, atk_hp, atk_hp_max, def_name, def_hp, def_hp_max
        ));

        rounds.push(RoundResult {
            round_number: round_num,
            attacker_roll: atk_roll,
            defender_roll: def_roll,
            attacker_damage: atk_dmg,
            defender_damage: def_dmg,
            attacker_hp_after: atk_hp,
            defender_hp_after: def_hp,
            chaos_event,
            attacker_passif: atk_passif,
            defender_passif: def_passif,
            message: round_msg,
        });

        // ── Check KO ──
        if atk_hp <= 0 || def_hp <= 0 {
            break;
        }
    }

    // ══════════════════════════════════════════════════════════════════
    // ── Determine winner ──
    // ══════════════════════════════════════════════════════════════════

    let total_rounds = rounds.len() as i32;
    let atk_hp_pct = if atk_hp_max > 0 {
        (atk_hp as f64 / atk_hp_max as f64 * 100.0) as i32
    } else {
        0
    };
    let def_hp_pct = if def_hp_max > 0 {
        (def_hp as f64 / def_hp_max as f64 * 100.0) as i32
    } else {
        0
    };

    let ko = atk_hp <= 0 || def_hp <= 0;

    // Determine winner/loser
    let (winner_id, loser_id, winner_pct, loser_pct, winner_coward) = if atk_hp <= 0 && def_hp <= 0
    {
        // Both KO at same time -> compare who had more HP% before last round
        // Treat as draw
        (None, None, atk_hp_pct, def_hp_pct, 1.0)
    } else if def_hp <= 0 {
        (
            Some(attacker.user_id.clone()),
            Some(defender.user_id.clone()),
            atk_hp_pct,
            def_hp_pct,
            coward_penalty_atk,
        )
    } else if atk_hp <= 0 {
        (
            Some(defender.user_id.clone()),
            Some(attacker.user_id.clone()),
            def_hp_pct,
            atk_hp_pct,
            coward_penalty_def,
        )
    } else if atk_hp_pct > def_hp_pct {
        // Timeout: highest HP% wins
        (
            Some(attacker.user_id.clone()),
            Some(defender.user_id.clone()),
            atk_hp_pct,
            def_hp_pct,
            coward_penalty_atk,
        )
    } else if def_hp_pct > atk_hp_pct {
        (
            Some(defender.user_id.clone()),
            Some(attacker.user_id.clone()),
            def_hp_pct,
            atk_hp_pct,
            coward_penalty_def,
        )
    } else {
        // Equal HP% -> draw
        (None, None, atk_hp_pct, def_hp_pct, 1.0)
    };

    let is_draw = winner_id.is_none();

    // ── Gains calculation based on HP% margin ──
    let hp_diff = (atk_hp_pct - def_hp_pct).abs();
    let (win_pct, lose_pct) = if hp_diff < 15 {
        (0.70, 0.60)
    } else if hp_diff <= 40 {
        (0.85, 0.80)
    } else {
        (1.00, 1.00)
    };

    // ── Resume des items utilises (prepend both winner + draw paths) ──
    let items_summary =
        build_items_summary(&atk_name, &def_name, special, defender_special, params);

    if is_draw {
        // ── Draw path ──
        let mut final_msg = format!("{}\n\n", start_msg);
        final_msg.push_str(&items_summary);
        for r in &rounds {
            final_msg.push_str(&r.message);
            final_msg.push_str("\n\n");
        }
        final_msg.push_str(pick_random(COMBAT_DRAW));

        if chaos_count > 0 {
            let chaos_list: Vec<String> = rounds
                .iter()
                .filter_map(|r| {
                    r.chaos_event
                        .as_ref()
                        .map(|ce| format!("{} {}", ce.emoji(), ce.label()))
                })
                .collect();
            final_msg.push_str(&format!(
                "\n\n\u{1f300} **Chaos ({})** : {}",
                chaos_count,
                chaos_list.join(", ")
            ));
        }

        return CombatResult {
            winner_id: None,
            loser_id: None,
            rounds,
            total_rounds,
            attacker_hp_final: atk_hp,
            defender_hp_final: def_hp,
            attacker_hp_max: atk_hp_max,
            defender_hp_max: def_hp_max,
            chaos_events_count: chaos_count,
            coins_won: 0,
            coins_lost_by_loser: 0,
            stolen_bonus: 0,
            vol_coins: vol_coins_total,
            message: final_msg,
            is_giant_killer: false,
            attacker_class_revealed,
            defender_class_revealed,
        };
    }

    // ── Winner path ──
    // Tous les calculs utilisent saturating_* pour eviter overflow/wrap sur
    // des mises proches de i64::MAX. Les coins sont clamp a [1, i64::MAX].
    let mise_f = mise as f64;
    let mut coins_won: i64 = ((mise_f * win_pct).clamp(0.0, i64::MAX as f64)) as i64;
    let coins_lost: i64 = ((mise_f * lose_pct).clamp(0.0, i64::MAX as f64)) as i64;

    if coins_won < 1 {
        coins_won = 1;
    }

    // Giant killer: 3+ level gap underdog winning
    let is_giant = if let (Some(ref wid), Some(ref lid)) = (&winner_id, &loser_id) {
        let winner_lvl = if *wid == attacker.user_id {
            attacker.level
        } else {
            defender.level
        };
        let loser_lvl = if *lid == attacker.user_id {
            attacker.level
        } else {
            defender.level
        };
        level_gap >= 3 && winner_lvl < loser_lvl
    } else {
        false
    };

    // Fourbe steal bonus
    let winner_class_name = if winner_id.as_deref() == Some(&attacker.user_id) {
        atk_class.name
    } else {
        def_class.name
    };
    let w_class = classes::get_class(winner_class_name);
    let stolen_bonus_val: i64 = if w_class.steal_bonus > 0.0 {
        ((mise_f * w_class.steal_bonus).clamp(0.0, i64::MAX as f64)) as i64
    } else {
        0
    };
    coins_won = coins_won.saturating_add(stolen_bonus_val);

    // Cowardice penalty
    coins_won = ((coins_won as f64 * winner_coward).clamp(0.0, i64::MAX as f64)) as i64;

    // Happy hour (multiplier est un i64 entier, typiquement 1 ou 2).
    coins_won = coins_won.saturating_mul(multiplier);

    // ── Build final message ──
    let winner_name = if winner_id.as_deref() == Some(&attacker.user_id) {
        &atk_name
    } else {
        &def_name
    };
    let loser_name = if winner_id.as_deref() == Some(&attacker.user_id) {
        &def_name
    } else {
        &atk_name
    };

    let mut final_msg = format!("{}\n\n", start_msg);
    final_msg.push_str(&items_summary);

    // Append round summaries
    for r in &rounds {
        final_msg.push_str(&r.message);
        final_msg.push_str("\n\n");
    }

    // Ending
    if ko {
        let ko_txt = fmt_template(
            pick_random(COMBAT_KO),
            &[("{perdant}", loser_name), ("{gagnant}", winner_name)],
        );
        final_msg.push_str(&ko_txt);
    } else {
        let timeout_txt = fmt_template(
            pick_random(COMBAT_TIMEOUT),
            &[
                ("{gagnant}", winner_name),
                ("{hp_g}", &winner_pct.to_string()),
                ("{hp_p}", &loser_pct.to_string()),
            ],
        );
        final_msg.push_str(&timeout_txt);
    }

    final_msg.push_str(&format!(
        "\n\u{1f4b0} {} empoche **{} coins** ! {} perd **{} coins** !",
        winner_name, coins_won, loser_name, coins_lost
    ));

    if is_giant {
        final_msg.push_str(&format!(
            "\n\u{1f525} **GIANT KILLER !** {} terrasse un adversaire de {} niveaux au-dessus ! +15 XP bonus !",
            winner_name, level_gap
        ));
    }

    if vol_coins_total > 0 {
        final_msg.push_str(&format!(
            "\n\u{1f4b0} Vol a la Tire total : +{} coins voles !",
            vol_coins_total
        ));
    }

    if stolen_bonus_val > 0 {
        final_msg.push_str(&format!(
            "\n\u{1f5e1}\u{fe0f} Bonus fourbe : +{} coins voles !",
            stolen_bonus_val
        ));
    }

    if happy_hour {
        final_msg.push_str("\n\u{1f389} **HAPPY HOUR** — Gains doubles !");
    }

    if winner_coward < 1.0 {
        final_msg.push_str("\n\u{1f414} Le gagnant est un lache notoire... -20% sur les gains !");
    }

    if level_gap >= 3 {
        let handicap_pct = ((1.0 - handicap) * 100.0) as i32;
        if stronger_is_attacker {
            final_msg.push_str(&format!(
                "\n\u{2696}\u{fe0f} Handicap matchmaking : {} a -{}% ATK",
                atk_name, handicap_pct
            ));
        } else if stronger_is_defender {
            final_msg.push_str(&format!(
                "\n\u{2696}\u{fe0f} Handicap matchmaking : {} a -{}% ATK",
                def_name, handicap_pct
            ));
        }
    }

    // Resume chaos en fin de combat pour visibilite.
    if chaos_count > 0 {
        let chaos_list: Vec<String> = rounds
            .iter()
            .filter_map(|r| {
                r.chaos_event
                    .as_ref()
                    .map(|ce| format!("{} {}", ce.emoji(), ce.label()))
            })
            .collect();
        final_msg.push_str(&format!(
            "\n\n\u{1f300} **Chaos ({})** : {}",
            chaos_count,
            chaos_list.join(", ")
        ));
    }

    CombatResult {
        winner_id: winner_id.map(Into::into),
        loser_id: loser_id.map(Into::into),
        rounds,
        total_rounds,
        attacker_hp_final: atk_hp,
        defender_hp_final: def_hp,
        attacker_hp_max: atk_hp_max,
        defender_hp_max: def_hp_max,
        chaos_events_count: chaos_count,
        coins_won,
        coins_lost_by_loser: coins_lost,
        stolen_bonus: 0,
        vol_coins: vol_coins_total,
        message: final_msg,
        is_giant_killer: is_giant,
        attacker_class_revealed,
        defender_class_revealed,
    }
}

// ══════════════════════════════════════════════════════════════════════
// ── Helpers ──
// ══════════════════════════════════════════════════════════════════════

/// Construit le resume "items utilises" affiche en tete du message combat.
/// Liste les specials (rage, bouclier, poison, etc.) utilises par chaque
/// camp pour que le joueur voie l impact mecanique du combat.
fn build_items_summary(
    atk_name: &str,
    def_name: &str,
    atk_special: Option<&str>,
    def_special: Option<&str>,
    params: &BalanceParams,
) -> String {
    let fmt = |who: &str, sp: &str| -> Option<String> {
        match sp {
            "rage" => Some(format!(
                "\u{1f525} **{who}** utilise **Rage** (+{}% ATK / -{}% DEF)",
                params.rage_atk_bonus_pct, params.rage_def_malus_pct
            )),
            "coup_traitre" => Some(format!(
                "\u{1f5e1}\u{fe0f} **{who}** utilise **Coup Traitre** (-{}% DEF adverse)",
                params.coup_traitre_def_malus_pct
            )),
            "bouclier" => Some(format!(
                "\u{1f6e1}\u{fe0f} **{who}** utilise **Bouclier** (+{}% DEF)",
                params.bouclier_def_bonus_pct
            )),
            "double_coup" => Some(format!(
                "\u{1f94a} **{who}** utilise **Double Coup** (2d20 par round)"
            )),
            "poison" => Some(format!(
                "\u{2620}\u{fe0f} **{who}** empoisonne l'adversaire (-{} HP/round)",
                params.poison_damage_per_round
            )),
            "antidote" => Some(format!(
                "\u{1f33f} **{who}** utilise **Antidote** (immunise au poison)"
            )),
            "explosion" => Some(format!(
                "\u{1f4a5} **{who}** utilise **Explosion** (-50% mise pour les deux)"
            )),
            "surprise" => Some(format!(
                "\u{1f631} **{who}** utilise **Attaque Surprise** (resolution auto)"
            )),
            "mindgame" => Some(format!(
                "\u{1f9e0} **{who}** utilise **Mindgame** (revele classe + HP)"
            )),
            _ => None,
        }
    };

    let mut lines: Vec<String> = Vec::new();
    if let Some(s) = atk_special.and_then(|sp| fmt(atk_name, sp)) {
        lines.push(s);
    }
    if let Some(s) = def_special.and_then(|sp| fmt(def_name, sp)) {
        lines.push(s);
    }
    if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n\n", lines.join("\n"))
    }
}

// ══════════════════════════════════════════════════════════════════════
// ── Tests ──
// ══════════════════════════════════════════════════════════════════════

#[cfg(test)]
#[path = "tests/combat.rs"]
mod tests;
