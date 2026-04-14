import type { RouteRecordRaw } from "vue-router";
import SetupPage from "../components/pages/SetupPage.vue";
import LoginPage from "../components/pages/LoginPage.vue";
import DashboardPage from "../components/pages/DashboardPage.vue";
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
import AnalyticsPage from "../components/pages/AnalyticsPage.vue";
import IaConfigPage from "../components/pages/IaConfigPage.vue";
import MembersPage from "../components/pages/MembersPage.vue";
import CoudePage from "../components/pages/CoudePage.vue";
import BlackjackPage from "../components/pages/BlackjackPage.vue";
import WalletPage from "../components/pages/WalletPage.vue";
import RbacPage from "../components/pages/RbacPage.vue";
import SettingsPage from "../components/pages/SettingsPage.vue";
import AuthCallbackPage from "../components/pages/AuthCallbackPage.vue";

export const routes: RouteRecordRaw[] = [
  { path: "/setup", name: "setup", component: SetupPage, meta: { public: true } },
  { path: "/login", name: "login", component: LoginPage, meta: { public: true } },
  { path: "/auth/callback", name: "auth-callback", component: AuthCallbackPage, meta: { public: true } },
  { path: "/", name: "dashboard", component: DashboardPage },
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
  { path: "/analytics", name: "analytics", component: AnalyticsPage },
  { path: "/ia-config", name: "ia-config", component: IaConfigPage },
  { path: "/coude", name: "coude", component: CoudePage },
  { path: "/blackjack", name: "blackjack", component: BlackjackPage },
  { path: "/wallet", name: "wallet", component: WalletPage },
  { path: "/rbac", name: "rbac", component: RbacPage },
  { path: "/settings", name: "settings", component: SettingsPage },
];
