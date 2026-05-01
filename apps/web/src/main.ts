import { createApp } from "vue";
import { createPinia } from "pinia";
import { createRouter, createWebHistory } from "vue-router";
import App from "./App.vue";
import { routes } from "./router";
import { useAuth } from "./composables/useAuth";
import { initAppData, resetAppInit } from "./composables/useAppInit";
import "./styles/global.css";

const router = createRouter({
  history: createWebHistory(),
  routes,
});

// En production, l'app a une URL API relative auto (window.location.origin)
// et le user n'a plus a saisir API URL / API key / Discord secret. La page
// Setup n'est plus le passage oblige - on va directement au Login.
// La page /setup reste accessible manuellement pour les superadmins qui
// veulent override la config (debug, instance multi-domaine).
import { setApiConfig, getApiConfig } from "./api/config";
function ensureProdConfig() {
  if (import.meta.env.PROD && !getApiConfig()) {
    // Bootstrap : config par defaut = origin courant + pas de Bearer (les
    // browser users s'auth via X-Discord-Token, pas via API key).
    setApiConfig({ api_url: window.location.origin, api_key: "" });
  }
}

router.beforeEach(async (to, _from, next) => {
  ensureProdConfig();
  const { user, hasConfig, checkSession } = useAuth();
  await checkSession();
  // Setup uniquement si vraiment pas de config (cas dev sans defaults).
  if (!hasConfig.value && to.name !== "setup") { next({ name: "setup" }); return; }
  if (!to.meta.public && !user.value) { next({ name: "login" }); return; }
  if (user.value && (to.name === "login" || to.name === "setup")) { next({ name: "dashboard" }); return; }

  // Prefetch async des donnees stables apres login. Non bloquant : on next()
  // immediatement. Les composables singleton (useBotDefinitions, useBotEnabledStatus,
  // useComponentVisibility) auront leur cache rempli quand les pages les liront.
  if (user.value) {
    const { useGuildSelector } = await import("./composables/useGuildSelector");
    const { selectedGuildId } = useGuildSelector();
    if (selectedGuildId.value) {
      void initAppData(selectedGuildId.value);
    }
  } else {
    resetAppInit();
  }
  next();
});

const app = createApp(App);
app.use(createPinia());
app.use(router);
app.mount("#app");
