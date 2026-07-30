import type { RbacRole } from "@/types";

/**
 * Registry des composants/boutons UI gardables par role applicatif.
 *
 * Chaque entree definit :
 *   - key : identifiant stable utilise en BDD (ne pas renommer apres deploy)
 *   - label : libelle affiche dans la grille de configuration
 *   - category : regroupement dans la grille
 *   - minRole : role MINIMAL par defaut pour voir le composant
 *
 * Comportement :
 *   - Si pas d'override en BDD pour (role, key) -> on applique : role >= minRole.
 *   - Si override en BDD -> il prevaut (true=visible, false=cache).
 *   - Le superadmin (SUPERADMIN_USER_IDS) bypass tout et voit tout.
 *   - Owner ne peut JAMAIS etre cache (la grille de config force visible=true pour Owner).
 */
export interface ComponentDef {
  key: string;
  label: string;
  category: string;
  minRole: RbacRole;
}

export const ROLE_RANK: Record<RbacRole, number> = {
  viewer: 0,
  moderator: 1,
  admin: 2,
  owner: 3,
};

export const ROLES_ORDER: RbacRole[] = ["viewer", "moderator", "admin", "owner"];

export const COMPONENT_REGISTRY: ComponentDef[] = [
  // ── Dashboard (clés alignees sur DashboardPage.vue) ──
  { key: "general.stats", label: "Bouton Statistiques", category: "Dashboard", minRole: "viewer" },
  { key: "general.modstats", label: "Bouton Stats modération", category: "Dashboard", minRole: "moderator" },
  { key: "moderation.hub", label: "Bouton Modération", category: "Dashboard", minRole: "moderator" },
  { key: "security.hub", label: "Bouton Sécurité", category: "Dashboard", minRole: "admin" },
  { key: "security.audit", label: "Bouton Audit", category: "Dashboard", minRole: "moderator" },
  // ── Plateforme jeux Nexus ──
  // Clef unique de l'univers Nexus : elle pilote a la fois l'affichage de
  // l'onglet et, cote serveur, l'autorisation de la passerelle /nexus-api/
  // (gate `nexus.access` interroge par nginx via auth_request).
  // Droit d'ACCES a la passerelle /nexus-api : c'est LE gate interroge par
  // nginx (auth_request) avant chaque appel. Ce n'est pas une entree de
  // navigation, mais il doit figurer ici, sinon l'owner n'a aucun moyen de
  // l'ouvrir a un autre role depuis la page RBAC.
  // Un droit par journal : les anciens salons Discord avaient chacun leurs
  // permissions, on garde cette granularite (ex: un moderateur voit le vocal
  // et les messages, mais pas les commandes admin).
  { key: "logs.journal.members", label: "Journal — Membres", category: "Journaux", minRole: "moderator" },
  { key: "logs.journal.profiles", label: "Journal — Profils et roles", category: "Journaux", minRole: "moderator" },
  { key: "logs.journal.voice", label: "Journal — Vocal", category: "Journaux", minRole: "moderator" },
  { key: "logs.journal.messages", label: "Journal — Messages", category: "Journaux", minRole: "moderator" },
  { key: "logs.journal.server", label: "Journal — Serveur", category: "Journaux", minRole: "admin" },
  { key: "logs.journal.admin", label: "Journal — Commandes admin", category: "Journaux", minRole: "admin" },
  { key: "logs.journal.anomalies", label: "Journal — Anomalies", category: "Journaux", minRole: "admin" },
  { key: "nexus.access", label: "Nexus — Acces a la plateforme", category: "Nexus", minRole: "admin" },
  { key: "nexus.servers", label: "Nexus — Serveurs de jeu", category: "Nexus", minRole: "admin" },
  { key: "nexus.economy", label: "Nexus — Economie", category: "Nexus", minRole: "moderator" },
  { key: "nexus.coude", label: "Nexus — Coude", category: "Nexus", minRole: "moderator" },
  { key: "nexus.config", label: "Nexus — Configuration", category: "Nexus", minRole: "admin" },
  { key: "system.rbac", label: "Bouton RBAC", category: "Dashboard", minRole: "owner" },
  { key: "system.systemops", label: "Bouton SystemOps", category: "Dashboard", minRole: "admin" },
  { key: "system.server_health", label: "Bouton État serveur", category: "Dashboard", minRole: "admin" },
  { key: "config.server-security", label: "Bouton Sécurité serveur", category: "Dashboard", minRole: "admin" },
  { key: "system.members", label: "Bouton Membres", category: "Dashboard", minRole: "moderator" },
  { key: "config.ai-dataset", label: "Bouton Dataset IA", category: "Dashboard", minRole: "admin" },

  // ── Dashboard : communaute ──
  { key: "community.announcements", label: "Bouton Annonces planifiées", category: "Dashboard", minRole: "admin" },
  { key: "community.confessions", label: "Bouton Confessions (modération)", category: "Dashboard", minRole: "admin" },

  // ── Dashboard : modération (sous-pages) ──
  // Strikes / Notes / Rappels / Preuves / Reviews ont ete fusionnes
  // dans /moderation (onglets Suivi utilisateur + Workflow).
  { key: "moderation.rules", label: "Bouton Règles", category: "Dashboard", minRole: "moderator" },
  { key: "moderation.members", label: "Bouton Membres", category: "Dashboard", minRole: "moderator" },
  { key: "moderation.name-history", label: "Bouton Historique noms", category: "Dashboard", minRole: "moderator" },

  // ── Dashboard : sécurité ──
  { key: "security.automod", label: "Bouton Automod", category: "Dashboard", minRole: "admin" },

  // ── Dashboard : community ──
  { key: "community.welcome", label: "Bouton Welcome", category: "Dashboard", minRole: "admin" },
  { key: "community.tickets", label: "Bouton Tickets", category: "Dashboard", minRole: "moderator" },
  { key: "community.voice-channels", label: "Bouton Salons vocaux", category: "Dashboard", minRole: "moderator" },
  { key: "community.voice-themes", label: "Bouton Thèmes vocaux", category: "Dashboard", minRole: "admin" },
  { key: "community.role-panels", label: "Bouton Role panels", category: "Dashboard", minRole: "admin" },
  { key: "community.levels", label: "Bouton Niveaux", category: "Dashboard", minRole: "viewer" },
  { key: "community.levels-config", label: "Bouton Config niveaux", category: "Dashboard", minRole: "admin" },
  { key: "community.sponsorships", label: "Bouton Parrainages", category: "Dashboard", minRole: "moderator" },
  { key: "community.temp-roles", label: "Bouton Rôles temporaires", category: "Dashboard", minRole: "moderator" },

  // ── Dashboard : administration ──
  { key: "rotation.dashboard", label: "Bouton Administrateur tournant", category: "Dashboard", minRole: "admin" },

  // ── Dashboard : logs / jeux ──
  { key: "logs.system", label: "Logs techniques (bot/API/worker)", category: "Dashboard", minRole: "admin" },

  // ── Dashboard : config (admin) ──
  { key: "config.components", label: "Bouton Config composants", category: "Dashboard", minRole: "admin" },
  { key: "config.rbac", label: "Bouton Config RBAC", category: "Dashboard", minRole: "owner" },
  { key: "config.system-ops", label: "Bouton System Ops", category: "Dashboard", minRole: "admin" },
  { key: "config.server-health", label: "Bouton État serveur (config)", category: "Dashboard", minRole: "admin" },
  { key: "config.guild-backup", label: "Bouton Sauvegardes serveur", category: "Dashboard", minRole: "admin" },
  { key: "config.system-logs", label: "Bouton Logs système (bots/workers/API/WS)", category: "Dashboard", minRole: "admin" },

  // ── Docker ──
  { key: "docker.section", label: "Section Docker (visibilité)", category: "Docker", minRole: "admin" },
  { key: "docker.action.start", label: "Démarrer un conteneur", category: "Docker", minRole: "owner" },
  { key: "docker.action.stop", label: "Arrêter un conteneur", category: "Docker", minRole: "owner" },
  { key: "docker.action.restart", label: "Redémarrer un conteneur", category: "Docker", minRole: "owner" },
  { key: "docker.action.remove_container", label: "Supprimer un conteneur", category: "Docker", minRole: "owner" },
  { key: "docker.action.remove_image", label: "Supprimer une image", category: "Docker", minRole: "owner" },
  { key: "docker.action.remove_volume", label: "Supprimer un volume", category: "Docker", minRole: "owner" },
  { key: "docker.action.logs", label: "Voir les logs", category: "Docker", minRole: "admin" },
  { key: "docker.prune.containers", label: "Nettoyage : conteneurs", category: "Docker", minRole: "owner" },
  { key: "docker.prune.images", label: "Nettoyage : images", category: "Docker", minRole: "owner" },
  { key: "docker.prune.volumes", label: "Nettoyage : volumes", category: "Docker", minRole: "owner" },
  { key: "docker.prune.networks", label: "Nettoyage : réseaux", category: "Docker", minRole: "owner" },
  { key: "docker.prune.system", label: "Nettoyage système complet", category: "Docker", minRole: "owner" },

  // ── Modération ──
  { key: "moderation.purge", label: "Purge messages", category: "Modération", minRole: "moderator" },
  { key: "moderation.bulk_unban", label: "Débannissement en masse", category: "Modération", minRole: "owner" },
  { key: "moderation.delete_action", label: "Supprimer action", category: "Modération", minRole: "admin" },

  // ── Nettoyages base de données (owner uniquement) ──
  { key: "db.purge.audit_logs", label: "Vider les audit logs (DB)", category: "Nettoyages DB", minRole: "owner" },
  { key: "db.purge.security_events", label: "Vider les events sécurité (DB)", category: "Nettoyages DB", minRole: "owner" },
  { key: "db.purge.voice_history", label: "Vider l'historique vocal (DB)", category: "Nettoyages DB", minRole: "owner" },
  { key: "db.purge.voice_channel", label: "Purger un salon vocal archivé (DB)", category: "Nettoyages DB", minRole: "owner" },

  // ── Membres ──
  { key: "members.reset", label: "Reset complet d'un membre", category: "Membres", minRole: "owner" },
  { key: "members.surveillance", label: "Onglet Surveillance", category: "Membres", minRole: "moderator" },

  // ── RBAC ──
  { key: "rbac.grant", label: "Attribuer un rôle", category: "RBAC", minRole: "owner" },
  { key: "rbac.revoke", label: "Révoquer un rôle", category: "RBAC", minRole: "owner" },
  { key: "rbac.visibility", label: "Configurer visibilité composants", category: "RBAC", minRole: "owner" },

  // ── Exports ──
  { key: "exports.create", label: "Créer un export", category: "Exports", minRole: "moderator" },

  // ── Composants bots ──
  { key: "components.toggle", label: "Activer/désactiver un composant", category: "Configuration", minRole: "admin" },
];

export function componentByKey(key: string): ComponentDef | undefined {
  return COMPONENT_REGISTRY.find((c) => c.key === key);
}

export function categories(): string[] {
  const set = new Set<string>();
  for (const c of COMPONENT_REGISTRY) set.add(c.category);
  return Array.from(set);
}
