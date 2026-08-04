import { computed, ref, watch } from "vue";
import type { AccessMode, PlanItem, PlannedKind } from "@/services/guildStructureService";

/// Les quatre intentions d'accès, dans l'ordre croissant de droits. Le libellé
/// dit ce que la personne pourra FAIRE, pas quels bits sont posés — c'est tout
/// l'intérêt de ne pas exposer les permissions Discord telles quelles.
export const ACCESS_MODES: { value: AccessMode; label: string; icon: string; hint: string }[] = [
  { value: "denied", label: "Aucun accès", icon: "🚫", hint: "Le salon n'apparaît pas" },
  { value: "read", label: "Lecture seule", icon: "👁", hint: "Voit et lit, ne peut pas s'exprimer" },
  { value: "write", label: "Participation", icon: "✍️", hint: "Écrit / parle normalement" },
  { value: "moderate", label: "Modération", icon: "🛡️", hint: "Participation + outils de modération" },
];

/// Métadonnées d'affichage par type de salon. L'icône reprend celle que Discord
/// met devant le nom : c'est le seul repère visuel que les gens connaissent
/// déjà, autant ne pas en inventer un autre.
export const KIND_META: Record<PlannedKind, { label: string; icon: string; hint: string }> = {
  category: { label: "Catégorie", icon: "📁", hint: "Regroupe des salons" },
  text: { label: "Salon écrit", icon: "#", hint: "Discussions par messages" },
  voice: { label: "Salon vocal", icon: "🔊", hint: "Conversations audio" },
  announcement: { label: "Annonces", icon: "📣", hint: "Publiable vers d'autres serveurs" },
  stage: { label: "Conférence", icon: "🎤", hint: "Scène : quelques orateurs, du public" },
  forum: { label: "Forum", icon: "💬", hint: "Fils de discussion par sujet" },
};

/// Types proposables comme enfants d'une catégorie (tout sauf une catégorie,
/// que Discord refuse d'imbriquer).
export const CHILD_KINDS: PlannedKind[] = ["text", "voice", "announcement", "stage", "forum"];

/// Un modèle prêt à poser : le point de départ que Discord ne donne pas. Créer
/// « un espace support correct » demande sinon de deviner quels salons ouvrir,
/// lesquels rendre privés, et de répéter l'opération salon par salon.
export interface BuilderTemplate {
  id: string;
  label: string;
  icon: string;
  description: string;
  build: (uid: () => string) => PlanItem[];
}

export const TEMPLATES: BuilderTemplate[] = [
  {
    id: "communaute",
    label: "Espace communauté",
    icon: "🏡",
    description: "Accueil, règles, discussion générale et deux vocaux.",
    build: (uid) => {
      const cat = uid();
      return [
        { key: cat, name: "Communauté", kind: "category" },
        { key: uid(), name: "accueil", kind: "text", parent_key: cat, topic: "Bienvenue !" },
        { key: uid(), name: "règles", kind: "text", parent_key: cat, slowmode: 21600 },
        { key: uid(), name: "général", kind: "text", parent_key: cat },
        { key: uid(), name: "Salon vocal", kind: "voice", parent_key: cat },
        { key: uid(), name: "Détente", kind: "voice", parent_key: cat, user_limit: 10 },
      ];
    },
  },
  {
    id: "support",
    label: "Support & tickets",
    icon: "🎫",
    description: "Un salon d'ouverture public, le reste réservé à l'équipe.",
    build: (uid) => {
      const cat = uid();
      return [
        { key: cat, name: "Support", kind: "category" },
        { key: uid(), name: "ouvrir-un-ticket", kind: "text", parent_key: cat },
        { key: uid(), name: "faq", kind: "text", parent_key: cat, slowmode: 21600 },
        { key: uid(), name: "tickets-archives", kind: "text", parent_key: cat, private: true },
      ];
    },
  },
  {
    id: "staff",
    label: "Espace équipe",
    icon: "🛡️",
    description: "Entièrement privé : coordination, journaux et vocal d'équipe.",
    build: (uid) => {
      const cat = uid();
      return [
        { key: cat, name: "Équipe", kind: "category" },
        { key: uid(), name: "coordination", kind: "text", parent_key: cat, private: true },
        { key: uid(), name: "journaux", kind: "text", parent_key: cat, private: true },
        { key: uid(), name: "Réunion", kind: "voice", parent_key: cat, private: true },
      ];
    },
  },
  {
    id: "gaming",
    label: "Espace jeu",
    icon: "🎮",
    description: "Recherche de joueurs, partage de clips et trois vocaux d'équipe.",
    build: (uid) => {
      const cat = uid();
      return [
        { key: cat, name: "Jeu", kind: "category" },
        { key: uid(), name: "recherche-de-joueurs", kind: "text", parent_key: cat },
        { key: uid(), name: "clips", kind: "text", parent_key: cat },
        { key: uid(), name: "Équipe 1", kind: "voice", parent_key: cat, user_limit: 5 },
        { key: uid(), name: "Équipe 2", kind: "voice", parent_key: cat, user_limit: 5 },
        { key: uid(), name: "Vocal libre", kind: "voice", parent_key: cat },
      ];
    },
  },
  {
    id: "evenements",
    label: "Événements",
    icon: "📅",
    description: "Annonces, inscriptions et une scène pour les prises de parole.",
    build: (uid) => {
      const cat = uid();
      return [
        { key: cat, name: "Événements", kind: "category" },
        { key: uid(), name: "annonces", kind: "announcement", parent_key: cat },
        { key: uid(), name: "inscriptions", kind: "text", parent_key: cat },
        { key: uid(), name: "Scène", kind: "stage", parent_key: cat },
      ];
    },
  },
];

/// Reproduit la normalisation appliquée par Discord aux salons écrits, pour que
/// l'aperçu affiche le nom réel et non celui qu'on a tapé.
export function previewName(name: string, kind: PlannedKind): string {
  const trimmed = name.trim();
  if (kind !== "text" && kind !== "announcement" && kind !== "forum") {
    return trimmed.slice(0, 100);
  }
  return trimmed
    .toLowerCase()
    .replace(/[^\p{L}\p{N}]/gu, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 90);
}

/// Limite de participants exploitable, ou `null` pour « illimité ».
///
/// `v-model.number` rend la chaîne vide quand l'utilisateur efface le champ —
/// or l'effacer est précisément la façon de dire « pas de limite ». On traite
/// donc le vide comme une absence de limite, jamais comme une erreur de saisie.
export function normalizedUserLimit(raw: unknown): number | null {
  if (raw === null || raw === undefined || raw === "") return null;
  const n = Number(raw);
  return Number.isFinite(n) ? Math.trunc(n) : null;
}

/// Erreurs bloquantes du plan, dans les mêmes termes que le domaine Rust. Le
/// serveur revalide tout : ce contrôle-ci n'existe que pour ne pas faire
/// découvrir une faute de frappe après un aller-retour réseau.
export function planErrors(items: PlanItem[], guildId?: string): string[] {
  const errs: string[] = [];
  const seen = new Set<string>();

  for (const it of items) {
    const rules = it.access ?? [];
    const roles = rules.map((r) => r.role_id);
    if (new Set(roles).size !== roles.length) {
      errs.push(`« ${it.name.trim() || "(sans nom)"} » définit deux fois l'accès du même rôle.`);
    }
    if (it.private && guildId && roles.includes(guildId)) {
      errs.push(
        `« ${it.name.trim() || "(sans nom)"} » est marqué privé ET donne un accès à @everyone : choisissez l'un ou l'autre.`,
      );
    }
  }

  for (const it of items) {
    const shown = it.name.trim() || "(sans nom)";
    if (!it.name.trim()) {
      errs.push(`Un élément ${KIND_META[it.kind].label.toLowerCase()} n'a pas de nom.`);
      continue;
    }
    if (!previewName(it.name, it.kind)) {
      errs.push(`« ${shown} » ne contient aucun caractère utilisable pour un salon.`);
    }
    if (it.name.trim().length > 100) {
      errs.push(`Le nom « ${shown} » dépasse 100 caractères.`);
    }
    const scope = `${previewName(it.name, it.kind)}|${it.kind}|${it.parent_key ?? ""}`;
    if (seen.has(scope)) {
      errs.push(`Deux salons « ${shown} » du même type au même endroit.`);
    }
    seen.add(scope);
  }

  // Un parent qui ne désigne aucune catégorie du plan fait rejeter le plan
  // ENTIER côté serveur. Le dire ici évite de découvrir l'échec après coup,
  // notamment après une application partielle qui a retiré la catégorie.
  const keys = new Set(items.map((i) => i.key));
  for (const it of items) {
    if (it.parent_key && !keys.has(it.parent_key)) {
      errs.push(
        `« ${it.name.trim() || "(sans nom)"} » n'a plus de catégorie parente : rattachez-le ou retirez-le.`,
      );
    }
    const limit = normalizedUserLimit(it.user_limit);
    if (limit !== null && (limit < 0 || limit > 99)) {
      errs.push(`La limite de participants de « ${it.name.trim()} » doit être comprise entre 0 et 99.`);
    }
  }

  for (const cat of items.filter((i) => i.kind === "category")) {
    const children = items.filter((i) => i.parent_key === cat.key).length;
    if (children > 50) {
      errs.push(`La catégorie « ${cat.name} » contient ${children} salons (maximum 50).`);
    }
  }
  if (items.length > 100) {
    errs.push(`Le plan contient ${items.length} éléments (maximum 100).`);
  }
  return errs;
}

/// Pose (ou remplace) l'accès d'un rôle sur un salon. Fonction libre : elle sert
/// aussi bien au composable qu'à l'éditeur d'accès, qui manipule l'élément
/// réactif directement.
export function setAccess(item: PlanItem, roleId: string, mode: AccessMode) {
  if (!item.access) item.access = [];
  const existing = item.access.find((a) => a.role_id === roleId);
  if (existing) existing.mode = mode;
  else item.access.push({ role_id: roleId, mode });
}

export function removeAccess(item: PlanItem, roleId: string) {
  item.access = (item.access ?? []).filter((a) => a.role_id !== roleId);
}

/// État du constructeur : le plan en cours de composition.
///
/// Volontairement NON partagé au scope module — deux onglets du constructeur
/// composant le même plan à l'insu l'un de l'autre serait une surprise, pas une
/// fonctionnalité.
/// Clé de stockage du plan, par serveur. Un plan composé pour un serveur n'a
/// aucun sens sur un autre (les règles d'accès portent des IDs de rôles), donc
/// chacun garde le sien.
function storageKey(guildId: string): string {
  return `sentinel:server-builder:${guildId}`;
}

export function useServerBuilder(guildId?: () => string | null | undefined) {
  const items = ref<PlanItem[]>([]);
  let counter = 0;
  const uid = () => `k${++counter}`;

  /// Reprend la numérotation au-delà des clés restaurées, sinon un ajout
  /// réutiliserait une clé existante et deux salons deviendraient
  /// indiscernables (parents inclus).
  function resumeCounter() {
    for (const item of items.value) {
      const n = Number(/^k(\d+)$/.exec(item.key)?.[1]);
      if (Number.isInteger(n) && n > counter) counter = n;
    }
  }

  /// Sauvegarde le plan courant. `sessionStorage` et non `localStorage` : un
  /// plan est un brouillon de session, pas une préférence à retrouver des
  /// semaines plus tard.
  function persist() {
    const gid = guildId?.();
    if (!gid) return;
    try {
      if (items.value.length === 0) sessionStorage.removeItem(storageKey(gid));
      else sessionStorage.setItem(storageKey(gid), JSON.stringify(items.value));
    } catch {
      // Quota plein ou stockage refusé : la sauvegarde est un confort, son
      // échec ne doit jamais empêcher de composer un plan.
    }
  }

  /// Recharge le plan du serveur courant (vide si aucun).
  function restore() {
    const gid = guildId?.();
    items.value = [];
    counter = 0;
    if (!gid) return;
    try {
      const raw = sessionStorage.getItem(storageKey(gid));
      const parsed = raw ? JSON.parse(raw) : null;
      if (Array.isArray(parsed)) items.value = parsed as PlanItem[];
    } catch {
      items.value = [];
    }
    resumeCounter();
  }

  const categories = computed(() => items.value.filter((i) => i.kind === "category"));
  /// Salons sans catégorie : ils atterriront à la racine du serveur.
  const rootChannels = computed(() =>
    items.value.filter((i) => i.kind !== "category" && !i.parent_key),
  );
  const childrenOf = (key: string) => items.value.filter((i) => i.parent_key === key);

  const errors = computed(() => planErrors(items.value, guildId?.() ?? undefined));
  const isEmpty = computed(() => items.value.length === 0);
  const canApply = computed(() => !isEmpty.value && errors.value.length === 0);

  const summary = computed(() => ({
    categories: categories.value.length,
    channels: items.value.filter((i) => i.kind !== "category").length,
    private: items.value.filter((i) => i.private).length,
  }));

  /// Renvoie l'élément TEL QU'IL VIT dans le plan (le proxy réactif), et non
  /// l'objet littéral : le modifier via la valeur renvoyée doit rafraîchir
  /// l'écran, sinon l'appelant écrit dans le vide.
  function push(item: PlanItem): PlanItem {
    items.value.push(item);
    return items.value[items.value.length - 1];
  }

  function addCategory(name = "Nouvelle catégorie") {
    return push({ key: uid(), name, kind: "category" });
  }

  function addChannel(kind: PlannedKind, parentKey?: string | null) {
    return push({
      key: uid(),
      name: kind === "voice" || kind === "stage" ? "Nouveau salon" : "nouveau-salon",
      kind,
      parent_key: parentKey ?? null,
    });
  }

  /// Retirer une catégorie retire aussi ses salons : les laisser derrière eux
  /// les enverrait à la racine du serveur, ce que personne n'a demandé.
  function remove(key: string) {
    items.value = items.value.filter((i) => i.key !== key && i.parent_key !== key);
  }

  function applyTemplate(template: BuilderTemplate) {
    items.value.push(...template.build(uid));
  }

  function reset() {
    items.value = [];
  }

  /// Rattache un salon : racine, catégorie du plan (`plan:<clé>`) ou catégorie
  /// déjà présente sur le serveur (`guild:<id>`).
  ///
  /// Déplacer un salon exigeait auparavant de le supprimer puis de le recréer
  /// ailleurs, en reperdant ses accès au passage.
  function setParent(item: PlanItem, target: string) {
    if (target.startsWith("plan:")) {
      item.parent_key = target.slice(5);
      item.parent_id = null;
    } else if (target.startsWith("guild:")) {
      item.parent_key = null;
      item.parent_id = target.slice(6);
    } else {
      item.parent_key = null;
      item.parent_id = null;
    }
  }

  /// Valeur du sélecteur de catégorie pour cet élément.
  function parentValue(item: PlanItem): string {
    if (item.parent_key) return `plan:${item.parent_key}`;
    if (item.parent_id) return `guild:${item.parent_id}`;
    return "";
  }

  // Toute évolution du plan est sauvegardée : c'est ce qui rend une navigation
  // accidentelle ou un rechargement sans conséquence.
  watch(items, persist, { deep: true });

  /// Retire du plan les éléments déjà créés, en reportant l'ID Discord des
  /// catégories créées sur leurs salons restants.
  ///
  /// Sans ce report, un échec partiel laissait des salons pointant une
  /// catégorie disparue du plan, et le serveur rejetait ensuite le plan entier
  /// — exactement l'inverse du « corrigez et relancez » annoncé.
  function dropCreated(created: { key: string; channel_id: string | null }[]) {
    const byKey = new Map(created.map((c) => [c.key, c.channel_id]));
    for (const item of items.value) {
      if (!item.parent_key) continue;
      const parentId = byKey.get(item.parent_key);
      if (parentId) {
        item.parent_key = null;
        item.parent_id = parentId;
      }
    }
    items.value = items.value.filter((i) => !byKey.has(i.key));
  }

  /// Plan nettoyé pour l'envoi : noms détourés, champs vides omis.
  ///
  /// `user_limit` est repassé au crible : `v-model.number` sur un champ vidé
  /// rend la chaîne vide, que le serveur refuse (`Option<u32>`) en rejetant
  /// tout le plan.
  function payload(): PlanItem[] {
    return items.value.map((i) => ({
      key: i.key,
      name: i.name.trim(),
      kind: i.kind,
      parent_key: i.parent_key || null,
      parent_id: i.parent_key ? null : i.parent_id || null,
      topic: i.topic?.trim() || null,
      slowmode: Number.isFinite(Number(i.slowmode)) ? Number(i.slowmode) : 0,
      user_limit: normalizedUserLimit(i.user_limit),
      nsfw: !!i.nsfw,
      private: !!i.private,
      access: (i.access ?? []).map((a) => ({ role_id: a.role_id, mode: a.mode })),
    }));
  }

  return {
    items,
    categories,
    rootChannels,
    childrenOf,
    errors,
    isEmpty,
    canApply,
    summary,
    addCategory,
    addChannel,
    setAccess,
    removeAccess,
    remove,
    dropCreated,
    setParent,
    parentValue,
    restore,
    applyTemplate,
    reset,
    payload,
  };
}
