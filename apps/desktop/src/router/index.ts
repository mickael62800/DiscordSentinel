import type { RouteRecordRaw } from "vue-router";
import SetupPage from "../components/pages/SetupPage.vue";
import LoginPage from "../components/pages/LoginPage.vue";
import DashboardPage from "../components/pages/DashboardPage.vue";
import LogsPage from "../components/pages/LogsPage.vue";
import LogsBotsPage from "../components/pages/LogsBotsPage.vue";
import LogsWorkersPage from "../components/pages/LogsWorkersPage.vue";
import LogsApiPage from "../components/pages/LogsApiPage.vue";
import LogsWebSocketPage from "../components/pages/LogsWebSocketPage.vue";
import InfractionsPage from "../components/pages/InfractionsPage.vue";
import RulesPage from "../components/pages/RulesPage.vue";
import BansPage from "../components/pages/BansPage.vue";
import ModerationPage from "../components/pages/ModerationPage.vue";
import SecurityPage from "../components/pages/SecurityPage.vue";
import TicketsPage from "../components/pages/TicketsPage.vue";
import VoiceChannelsPage from "../components/pages/VoiceChannelsPage.vue";
import ConductPage from "../components/pages/ConductPage.vue";
import BotConfigPage from "../components/pages/BotConfigPage.vue";
import WorkerConfigPage from "../components/pages/WorkerConfigPage.vue";
import WatchedUsersPage from "../components/pages/WatchedUsersPage.vue";
import LevelsPage from "../components/pages/LevelsPage.vue";
import RolePanelsPage from "../components/pages/RolePanelsPage.vue";
import AuditPage from "../components/pages/AuditPage.vue";
import AnalyticsPage from "../components/pages/AnalyticsPage.vue";
import AiTrainingPage from "../components/pages/AiTrainingPage.vue";
import IaConfigPage from "../components/pages/IaConfigPage.vue";
import SettingsPage from "../components/pages/SettingsPage.vue";

export const routes: RouteRecordRaw[] = [
  { path: "/setup", name: "setup", component: SetupPage, meta: { public: true } },
  { path: "/login", name: "login", component: LoginPage, meta: { public: true } },
  { path: "/", name: "dashboard", component: DashboardPage },
  { path: "/logs", name: "logs", component: LogsPage },
  { path: "/logs/bots", name: "logs-bots", component: LogsBotsPage },
  { path: "/logs/workers", name: "logs-workers", component: LogsWorkersPage },
  { path: "/logs/api", name: "logs-api", component: LogsApiPage },
  { path: "/logs/websocket", name: "logs-websocket", component: LogsWebSocketPage },
  { path: "/infractions", name: "infractions", component: InfractionsPage },
  { path: "/rules", name: "rules", component: RulesPage },
  { path: "/bans", name: "bans", component: BansPage },
  { path: "/moderation", name: "moderation", component: ModerationPage },
  { path: "/security", name: "security", component: SecurityPage },
  { path: "/tickets", name: "tickets", component: TicketsPage },
  { path: "/voice-channels", name: "voice-channels", component: VoiceChannelsPage },
  { path: "/conduct", name: "conduct", component: ConductPage },
  { path: "/bot-config", name: "bot-config", component: BotConfigPage },
  { path: "/worker-config", name: "worker-config", component: WorkerConfigPage },
  { path: "/watched-users", name: "watched-users", component: WatchedUsersPage },
  { path: "/levels", name: "levels", component: LevelsPage },
  { path: "/role-panels", name: "role-panels", component: RolePanelsPage },
  { path: "/audit", name: "audit", component: AuditPage },
  { path: "/analytics", name: "analytics", component: AnalyticsPage },
  { path: "/ai-training", name: "ai-training", component: AiTrainingPage },
  { path: "/ia-config", name: "ia-config", component: IaConfigPage },
  { path: "/settings", name: "settings", component: SettingsPage },
];
