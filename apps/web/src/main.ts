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

// Bootstrap : la config API par defaut (origin courant + pas de Bearer)
// suffit en prod. L'OAuth Discord est gere par le backend, le front n'a
// rien a saisir. Cette fonction garantit qu'au moins une config existe.
import { setApiConfig, getApiConfig } from "./api/config";
function ensureProdConfig() {
  if (!getApiConfig()) {
    setApiConfig({ api_url: window.location.origin, api_key: "" });
  }
}

router.beforeEach(async (to, _from, next) => {
  ensureProdConfig();
  const { user, checkSession } = useAuth();
  await checkSession();
  if (!to.meta.public && !user.value) { next({ name: "login" }); return; }
  if (user.value && to.name === "login") { next({ name: "dashboard" }); return; }

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
