import type { RouteRecordRaw } from "vue-router";

// Pages eagerly chargees : critiques au boot (login flow, callback OAuth).
// Tout le reste est lazy-loaded -> chaque page = un chunk separe, charge
// uniquement a la 1ere navigation. Reduit ~70-80% le bundle initial.
import SetupPage from "../components/pages/SetupPage.vue";
import LoginPage from "../components/pages/LoginPage.vue";
import AuthCallbackPage from "../components/pages/AuthCallbackPage.vue";
import DashboardPage from "../components/pages/DashboardPage.vue";

export const routes: RouteRecordRaw[] = [
  // ── Public / boot critique (eager) ──
  { path: "/setup", name: "setup", component: SetupPage, meta: { public: true } },
  { path: "/login", name: "login", component: LoginPage, meta: { public: true } },
  { path: "/auth/callback", name: "auth-callback", component: AuthCallbackPage, meta: { public: true } },

  // Dashboard reste eager : c'est la 1ere page apres login, autant l'avoir
  // dans le bundle initial pour eviter un flash de loader.
  { path: "/", name: "dashboard", component: DashboardPage },

  // ── Stats / Audit ──
  { path: "/stats", name: "stats", component: () => import("../components/pages/StatsPage.vue") },
  { path: "/modstats", name: "modstats", component: () => import("../components/pages/ModstatsPage.vue") },
  { path: "/logs", name: "logs", component: () => import("../components/pages/LogsPage.vue") },
  { path: "/system-logs", name: "system-logs", component: () => import("../components/pages/SystemLogsPage.vue") },
  { path: "/audit", name: "audit", component: () => import("../components/pages/AuditPage.vue") },
  { path: "/name-history", name: "name-history", component: () => import("../components/pages/NameHistoryPage.vue") },

  // ── Modération ──
  { path: "/moderation", name: "moderation", component: () => import("../components/pages/ModerationHubPage.vue") },
  { path: "/rules", name: "rules", component: () => import("../components/pages/RulesPage.vue") },
  { path: "/strikes", name: "strikes", component: () => import("../components/pages/StrikesPage.vue") },
  { path: "/notes", name: "notes", component: () => import("../components/pages/NotesPage.vue") },
  { path: "/reminders", name: "reminders", component: () => import("../components/pages/RemindersPage.vue") },
  { path: "/evidence", name: "evidence", component: () => import("../components/pages/EvidencePage.vue") },
  { path: "/review", name: "review", component: () => import("../components/pages/ReviewPage.vue") },
  { path: "/automod", name: "automod", component: () => import("../components/pages/AutomodPage.vue") },

  // ── Sécurité ──
  { path: "/security", name: "security", component: () => import("../components/pages/SecurityPage.vue") },

  // ── Communauté ──
  { path: "/welcome", name: "welcome", component: () => import("../components/pages/WelcomePage.vue") },
  { path: "/tickets", name: "tickets", component: () => import("../components/pages/TicketsPage.vue") },
  { path: "/voice-channels", name: "voice-channels", component: () => import("../components/pages/VoiceChannelsPage.vue") },
  { path: "/voice-themes", name: "voice-themes", component: () => import("../components/pages/VoiceThemesPage.vue") },
  { path: "/role-panels", name: "role-panels", component: () => import("../components/pages/RolePanelsPage.vue") },
  { path: "/role-panels/new", name: "role-panel-new", component: () => import("../components/pages/RolePanelEditPage.vue") },
  { path: "/discord-roles", name: "discord-roles", component: () => import("../components/pages/DiscordRolesPage.vue") },
  { path: "/levels", name: "levels", component: () => import("../components/pages/LevelsPage.vue") },
  { path: "/levels-config", name: "levels-config", component: () => import("../components/pages/LevelsConfigPage.vue") },
  { path: "/sponsorships", name: "sponsorships", component: () => import("../components/pages/SponsorshipsPage.vue") },
  { path: "/temp-roles", name: "temp-roles", component: () => import("../components/pages/TempRolesPage.vue") },
  { path: "/members", name: "members", component: () => import("../components/pages/MembersPage.vue") },
  { path: "/conduct", redirect: "/members" },
  { path: "/watched-users", redirect: "/members" },

  // ── Jeux ──
  { path: "/games", name: "games", component: () => import("../components/pages/GamesPage.vue") },
  { path: "/coude", name: "coude", component: () => import("../components/pages/CoudeHubPage.vue") },
  { path: "/coude/social", name: "coude-social", component: () => import("../components/pages/CoudeSocialPage.vue") },
  { path: "/taunts", name: "taunts", component: () => import("../components/pages/TauntsConfigPage.vue") },
  { path: "/coude/taunts", redirect: "/taunts" },
  { path: "/blackjack", name: "blackjack", component: () => import("../components/pages/BlackjackPage.vue") },
  { path: "/slot", name: "slot", component: () => import("../components/pages/SlotPage.vue") },
  { path: "/wheel", name: "wheel", component: () => import("../components/pages/WheelPage.vue") },
  { path: "/wallet", name: "wallet", component: () => import("../components/pages/WalletPage.vue") },
  { path: "/tournaments", name: "tournaments", component: () => import("../components/pages/TournamentPage.vue") },
  { path: "/game-portal", name: "game-portal", component: () => import("../components/pages/GamePortalPage.vue") },

  // ── Configuration / Admin ──
  { path: "/component-config", name: "component-config", component: () => import("../components/pages/ComponentConfigPage.vue") },
  { path: "/rbac", name: "rbac", component: () => import("../components/pages/RbacPage.vue") },
  { path: "/system/operations", name: "system-ops", component: () => import("../components/pages/SystemOpsPage.vue") },
  { path: "/server-health", name: "server-health", component: () => import("../components/pages/ServerHealthPage.vue") },
  { path: "/server-security", name: "server-security", component: () => import("../components/pages/ServerSecurityPage.vue") },
  { path: "/ai-dataset", name: "ai-dataset", component: () => import("../components/pages/AiDatasetPage.vue") },
  { path: "/settings", name: "settings", component: () => import("../components/pages/SettingsPage.vue") },
];
