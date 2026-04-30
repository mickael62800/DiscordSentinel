import type { RouteRecordRaw } from "vue-router";
import SetupPage from "../components/pages/SetupPage.vue";
import LoginPage from "../components/pages/LoginPage.vue";
import DashboardPage from "../components/pages/DashboardPage.vue";
import StatsPage from "../components/pages/StatsPage.vue";
import LogsPage from "../components/pages/LogsPage.vue";
import ModerationHubPage from "../components/pages/ModerationHubPage.vue";
import RulesPage from "../components/pages/RulesPage.vue";
import SecurityPage from "../components/pages/SecurityPage.vue";
import TicketsPage from "../components/pages/TicketsPage.vue";
import VoiceChannelsPage from "../components/pages/VoiceChannelsPage.vue";
import ComponentConfigPage from "../components/pages/ComponentConfigPage.vue";
import LevelsPage from "../components/pages/LevelsPage.vue";
import RolePanelsPage from "../components/pages/RolePanelsPage.vue";
import DiscordRolesPage from "../components/pages/DiscordRolesPage.vue";
import AuditPage from "../components/pages/AuditPage.vue";
import MembersPage from "../components/pages/MembersPage.vue";
import CoudePage from "../components/pages/CoudePage.vue";
import TauntsConfigPage from "../components/pages/TauntsConfigPage.vue";
import BlackjackPage from "../components/pages/BlackjackPage.vue";
import WalletPage from "../components/pages/WalletPage.vue";
import RbacPage from "../components/pages/RbacPage.vue";
import GamesPage from "../components/pages/GamesPage.vue";
import TournamentPage from "../components/pages/TournamentPage.vue";
import SettingsPage from "../components/pages/SettingsPage.vue";
import AuthCallbackPage from "../components/pages/AuthCallbackPage.vue";
import WelcomePage from "../components/pages/WelcomePage.vue";
import AutomodPage from "../components/pages/AutomodPage.vue";
import StrikesPage from "../components/pages/StrikesPage.vue";
import NotesPage from "../components/pages/NotesPage.vue";
import RemindersPage from "../components/pages/RemindersPage.vue";
import EvidencePage from "../components/pages/EvidencePage.vue";
import ReviewPage from "../components/pages/ReviewPage.vue";
import ModstatsPage from "../components/pages/ModstatsPage.vue";
import VoiceThemesPage from "../components/pages/VoiceThemesPage.vue";
import RolePanelEditPage from "../components/pages/RolePanelEditPage.vue";
import CoudeSocialPage from "../components/pages/CoudeSocialPage.vue";
import LevelsConfigPage from "../components/pages/LevelsConfigPage.vue";
import SponsorshipsPage from "../components/pages/SponsorshipsPage.vue";
import TempRolesPage from "../components/pages/TempRolesPage.vue";
import SystemOpsPage from "../components/pages/SystemOpsPage.vue";
import ServerHealthPage from "../components/pages/ServerHealthPage.vue";
import SlotPage from "../components/pages/SlotPage.vue";
import WheelPage from "../components/pages/WheelPage.vue";
import NameHistoryPage from "../components/pages/NameHistoryPage.vue";

export const routes: RouteRecordRaw[] = [
  { path: "/setup", name: "setup", component: SetupPage, meta: { public: true } },
  { path: "/login", name: "login", component: LoginPage, meta: { public: true } },
  { path: "/auth/callback", name: "auth-callback", component: AuthCallbackPage, meta: { public: true } },
  { path: "/", name: "dashboard", component: DashboardPage },
  { path: "/stats", name: "stats", component: StatsPage },
  { path: "/logs", name: "logs", component: LogsPage },
  { path: "/moderation", name: "moderation", component: ModerationHubPage },
  { path: "/rules", name: "rules", component: RulesPage },
  { path: "/security", name: "security", component: SecurityPage },
  { path: "/tickets", name: "tickets", component: TicketsPage },
  { path: "/voice-channels", name: "voice-channels", component: VoiceChannelsPage },
  { path: "/members", name: "members", component: MembersPage },
  { path: "/conduct", redirect: "/members" },
  { path: "/watched-users", redirect: "/members" },
  { path: "/component-config", name: "component-config", component: ComponentConfigPage },
  { path: "/levels", name: "levels", component: LevelsPage },
  { path: "/role-panels", name: "role-panels", component: RolePanelsPage },
  { path: "/discord-roles", name: "discord-roles", component: DiscordRolesPage },
  { path: "/audit", name: "audit", component: AuditPage },
  { path: "/coude", name: "coude", component: CoudePage },
  { path: "/taunts", name: "taunts", component: TauntsConfigPage },
  // Redirection de l'ancienne URL pour conserver les bookmarks.
  { path: "/coude/taunts", redirect: "/taunts" },
  { path: "/blackjack", name: "blackjack", component: BlackjackPage },
  { path: "/wallet", name: "wallet", component: WalletPage },
  { path: "/games", name: "games", component: GamesPage },
  { path: "/tournaments", name: "tournaments", component: TournamentPage },
  { path: "/rbac", name: "rbac", component: RbacPage },
  { path: "/welcome", name: "welcome", component: WelcomePage },
  { path: "/automod", name: "automod", component: AutomodPage },
  { path: "/strikes", name: "strikes", component: StrikesPage },
  { path: "/notes", name: "notes", component: NotesPage },
  { path: "/reminders", name: "reminders", component: RemindersPage },
  { path: "/evidence", name: "evidence", component: EvidencePage },
  { path: "/review", name: "review", component: ReviewPage },
  { path: "/modstats", name: "modstats", component: ModstatsPage },
  { path: "/voice-themes", name: "voice-themes", component: VoiceThemesPage },
  { path: "/role-panels/new", name: "role-panel-new", component: RolePanelEditPage },
  { path: "/coude/social", name: "coude-social", component: CoudeSocialPage },
  { path: "/levels-config", name: "levels-config", component: LevelsConfigPage },
  { path: "/sponsorships", name: "sponsorships", component: SponsorshipsPage },
  { path: "/temp-roles", name: "temp-roles", component: TempRolesPage },
  { path: "/system/operations", name: "system-ops", component: SystemOpsPage },
  { path: "/server-health", name: "server-health", component: ServerHealthPage },
  { path: "/slot", name: "slot", component: SlotPage },
  { path: "/wheel", name: "wheel", component: WheelPage },
  { path: "/name-history", name: "name-history", component: NameHistoryPage },
  { path: "/settings", name: "settings", component: SettingsPage },
];
