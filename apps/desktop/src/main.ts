import { createApp } from "vue";
import { createPinia } from "pinia";
import { createRouter, createWebHistory } from "vue-router";
import App from "./App.vue";
import { routes } from "./router";
// import { useAuth } from "./composables/useAuth";
import "./styles/global.css";

const router = createRouter({
  history: createWebHistory(),
  routes,
});

// Auth guard — TEMPORARILY DISABLED FOR TESTING
// TODO: re-enable auth guard before production
router.beforeEach(async (_to, _from, next) => {
  next();
});

// router.beforeEach(async (to, _from, next) => {
//   const { user, hasConfig, checkSession } = useAuth();
//   await checkSession();
//   if (!hasConfig.value && to.name !== "setup") { next({ name: "setup" }); return; }
//   if (!to.meta.public && !user.value) { next({ name: "login" }); return; }
//   if (user.value && (to.name === "login" || to.name === "setup")) { next({ name: "dashboard" }); return; }
//   next();
// });

const app = createApp(App);
app.use(createPinia());
app.use(router);
app.mount("#app");
