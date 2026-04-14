<script setup lang="ts">
import { onMounted } from "vue";
import { useRouter } from "vue-router";
import { setDiscordUser, setDiscordToken, type DiscordUser } from "@/api/config";
import { useAuthStore } from "@/stores/authStore";

const router = useRouter();
const store = useAuthStore();

onMounted(() => {
  // Le backend redirige ici avec les infos dans le FRAGMENT (#…) pour eviter
  // que le token n'apparaisse dans les logs serveur ou le referer.
  const hash = window.location.hash.startsWith("#")
    ? window.location.hash.slice(1)
    : window.location.hash;
  const params = new URLSearchParams(hash);

  const token = params.get("token");
  const id = params.get("id");
  const username = params.get("username");

  if (!token || !id || !username) {
    router.replace({ name: "login", query: { error: "callback_invalide" } });
    return;
  }

  const user: DiscordUser = {
    id,
    username,
    global_name: params.get("global_name") || null,
    avatar: params.get("avatar") || null,
  };

  setDiscordToken(token);
  setDiscordUser(user);

  // Nettoie l'URL (retire le fragment sensible) avant la prochaine nav.
  history.replaceState(null, "", window.location.pathname);

  // Injecte directement dans le store Pinia pour eviter un re-check async.
  store.$patch({ user, hasConfig: true, initialized: true, error: null });

  router.replace({ name: "dashboard" });
});
</script>

<template>
  <div class="callback-page">
    <p>Connexion en cours…</p>
  </div>
</template>

<style scoped>
.callback-page {
  width: 100vw;
  height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-secondary);
  font-size: 14px;
}
</style>
