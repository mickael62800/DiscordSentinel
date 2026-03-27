import type { RouteRecordRaw } from "vue-router";
import SetupPage from "../components/pages/SetupPage.vue";
import LoginPage from "../components/pages/LoginPage.vue";
import DashboardPage from "../components/pages/DashboardPage.vue";
import LogsPage from "../components/pages/LogsPage.vue";
import InfractionsPage from "../components/pages/InfractionsPage.vue";
import RulesPage from "../components/pages/RulesPage.vue";
import BansPage from "../components/pages/BansPage.vue";
import ModerationPage from "../components/pages/ModerationPage.vue";
import SecurityPage from "../components/pages/SecurityPage.vue";
import TicketsPage from "../components/pages/TicketsPage.vue";
import VoiceChannelsPage from "../components/pages/VoiceChannelsPage.vue";
import ConductPage from "../components/pages/ConductPage.vue";
import BotConfigPage from "../components/pages/BotConfigPage.vue";
import SettingsPage from "../components/pages/SettingsPage.vue";

export const routes: RouteRecordRaw[] = [
  { path: "/setup", name: "setup", component: SetupPage, meta: { public: true } },
  { path: "/login", name: "login", component: LoginPage, meta: { public: true } },
  { path: "/", name: "dashboard", component: DashboardPage },
  { path: "/logs", name: "logs", component: LogsPage },
  { path: "/infractions", name: "infractions", component: InfractionsPage },
  { path: "/rules", name: "rules", component: RulesPage },
  { path: "/bans", name: "bans", component: BansPage },
  { path: "/moderation", name: "moderation", component: ModerationPage },
  { path: "/security", name: "security", component: SecurityPage },
  { path: "/tickets", name: "tickets", component: TicketsPage },
  { path: "/voice-channels", name: "voice-channels", component: VoiceChannelsPage },
  { path: "/conduct", name: "conduct", component: ConductPage },
  { path: "/bot-config", name: "bot-config", component: BotConfigPage },
  { path: "/settings", name: "settings", component: SettingsPage },
];
