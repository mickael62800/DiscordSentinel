import { storeToRefs } from "pinia";
import { watch } from "vue";
import { useComponentVisibilityStore } from "@/stores/componentVisibilityStore";
import { useMyRoleStore } from "@/stores/myRoleStore";
import { useGuildSelector } from "./useGuildSelector";
import { getDiscordToken } from "@/api/config";

/**
 * Wrapper composable : delegue aux stores Pinia componentVisibility +
 * myRole, integre le watch sur selectedGuildId. API publique identique
 * a la version singleton precedente.
 */
export function useComponentVisibility() {
  const store = useComponentVisibilityStore();
  const myRoleStore = useMyRoleStore();
  const { selectedGuildId } = useGuildSelector();
  const { overrides, loaded, loading } = storeToRefs(store);
  const { role, isSuper } = storeToRefs(myRoleStore);

  watch(
    selectedGuildId,
    (gid) => {
      // Sans session, on ne charge RIEN. La visibilité des composants
      // d'administration n'a de sens que pour quelqu'un d'identifié, et
      // l'endpoint exige un jeton : l'appeler anonymement provoquait un 401,
      // que le client HTTP traduit en redirection vers /login.
      //
      // Le cas est apparu avec le mode mono-serveur : la guilde est désormais
      // imposée par la configuration, donc toujours renseignée. Auparavant
      // elle restait nulle pour un visiteur anonyme et la garde ci-dessous
      // était fournie par accident.
      if (gid && getDiscordToken()) void store.load(gid);
    },
    { immediate: true },
  );

  function visible(key: string): boolean {
    return store.visible(key);
  }

  async function reload(): Promise<void> {
    if (selectedGuildId.value) {
      store.invalidate();
      await store.load(selectedGuildId.value);
    }
  }

  return {
    visible,
    role,
    isSuper,
    loaded,
    loading,
    overrides,
    reload,
  };
}

/** Helper appele depuis useAppInit (router.beforeEach) pour prefetch. */
export async function preloadComponentVisibility(guildId: string): Promise<void> {
  await useComponentVisibilityStore().load(guildId);
}
