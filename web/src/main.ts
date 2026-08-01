import { createApp } from "vue";
import { MotionPlugin } from "@vueuse/motion";
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
import { loadSiteConfig } from "./siteConfig";
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
  // Un membre ordinaire n'a pas acces au back-office : l'y envoyer
  // afficherait une page vide ou une cascade de 403. Le role n'est pas
  // encore charge a cet instant, donc on route vers l'espace membre, qui
  // porte lui-meme un lien vers l'administration pour qui y a droit.
  if (user.value && to.name === "login") { next({ name: "membre" }); return; }

  // Prefetch async des donnees stables apres login. Non bloquant : on next()
  // immediatement. Les composables singleton (useBotDefinitions, useBotEnabledStatus,
  // useComponentVisibility) auront leur cache rempli quand les pages les liront.
  if (user.value) {
    const { useGuildSelector } = await import("./composables/useGuildSelector");
    const { selectedGuildId } = useGuildSelector();
    const gid = selectedGuildId.value;
    if (gid) {
      void initAppData(gid);

      // Guard RBAC au niveau route : empeche l'ouverture directe (par URL)
      // d'une page que le role de l'utilisateur ne voit pas. Le masquage des
      // tuiles seul ne protegeait pas l'acces direct. L'API applique deja le
      // RBAC (403), ce guard evite juste d'ouvrir une page qui echouerait.
      const { rbacKeyForPath } = await import("./composables/useDashboardSections");
      const key = rbacKeyForPath(to.path);
      if (key) {
        const { useComponentVisibilityStore } = await import(
          "./stores/componentVisibilityStore"
        );
        const store = useComponentVisibilityStore();
        // Charge (dedup/cache par guild) puis verifie. On n'enforce QUE si la
        // visibilite est chargee (role resolu) : sinon visible() renvoie false
        // faute de role et redirigerait a tort. Fail-open pendant le chargement.
        await store.load(gid);
        if (store.loaded && !store.visible(key)) {
          next({ name: "dashboard" });
          return;
        }
      }
    }
  } else {
    resetAppInit();
  }
  next();
});

const app = createApp(App);
app.use(createPinia());
app.use(router);
// Animations d'apparition au defilement (directive v-motion). Volontairement
// discretes : elles servent a guider la lecture de la page publique, pas a
// faire du spectacle. `prefers-reduced-motion` est respecte par la lib.
app.use(MotionPlugin);

// La configuration publique (guilde affichée, invitation Discord) est chargée
// AVANT le montage : la page membre la lit dès son `onMounted`, et l'attendre
// ici évite un premier rendu sans aucune section suivi d'un saut.
//
// Un échec n'empêche pas le montage : le site reste consultable, les sections
// publiques se masquent simplement.
loadSiteConfig().finally(() => app.mount("#app"));
