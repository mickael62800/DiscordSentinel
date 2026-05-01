import { computed, ref, watch } from "vue";
import type { ComponentVisibilityEntry, RbacRole } from "@/types";
import { componentByKey, ROLE_RANK } from "@/rbac/componentRegistry";
import { rbacService } from "@/services/rbacService";
import { useGuildSelector } from "./useGuildSelector";
import { useMyRole, preloadMyRole } from "./useMyRole";

// Etat partage (singleton via module scope) — chargement unique par guild.
const overrides = ref<ComponentVisibilityEntry[]>([]);
const loaded = ref(false);
const loading = ref(false);
let lastLoadedGuild: string | null = null;
let inFlight: Promise<void> | null = null;

async function load(guildId: string): Promise<void> {
  if (lastLoadedGuild === guildId && !inFlight) return;
  if (inFlight && lastLoadedGuild === guildId) return inFlight;

  loading.value = true;
  inFlight = (async () => {
    try {
      // visibility + myRole en parallele. myRole est lui-meme un singleton :
      // si deja charge pour cette guild, l'appel retourne instant le cache.
      const [list] = await Promise.all([
        rbacService.listComponentVisibility(guildId).catch(() => [] as ComponentVisibilityEntry[]),
        preloadMyRole(guildId),
      ]);
      overrides.value = list;
      lastLoadedGuild = guildId;
      loaded.value = true;
    } finally {
      loading.value = false;
      inFlight = null;
    }
  })();
  return inFlight;
}

export async function preloadComponentVisibility(guildId: string): Promise<void> {
  await load(guildId);
}

/**
 * Resout la visibilite d'un composant pour le role courant.
 *  - superadmin -> toujours true
 *  - override en BDD pour (role, key) -> sa valeur
 *  - sinon -> role >= minRole du registry
 */
function isVisibleFor(key: string, role: RbacRole | null, isSuper: boolean): boolean {
  if (isSuper) return true;
  if (!role) return false;
  const def = componentByKey(key);
  if (!def) {
    console.warn(`[visibility] composant inconnu: ${key}`);
    return true; // failsafe : on n'occulte pas par erreur de typo
  }
  const override = overrides.value.find((o) => o.component_key === key && o.role === role);
  if (override) return override.visible;
  return ROLE_RANK[role] >= ROLE_RANK[def.minRole];
}

export function useComponentVisibility() {
  const { selectedGuildId } = useGuildSelector();
  const { role, isSuper } = useMyRole();

  watch(
    selectedGuildId,
    (gid) => {
      if (gid && gid !== lastLoadedGuild) {
        lastLoadedGuild = null;
        loaded.value = false;
        void load(gid);
      }
    },
    { immediate: true },
  );

  function visible(key: string): boolean {
    return isVisibleFor(key, role.value, isSuper.value);
  }

  async function reload() {
    if (selectedGuildId.value) {
      lastLoadedGuild = null;
      await load(selectedGuildId.value);
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
