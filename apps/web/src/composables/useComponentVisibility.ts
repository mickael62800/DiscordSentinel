import { computed, ref, watch } from "vue";
import type { ComponentVisibilityEntry, MyRole, RbacRole } from "@/types";
import { componentByKey, ROLE_RANK } from "@/rbac/componentRegistry";
import { rbacService } from "@/services/rbacService";
import { useGuildSelector } from "./useGuildSelector";

// Etat partage (singleton via module scope) — chargement unique par guild.
const overrides = ref<ComponentVisibilityEntry[]>([]);
const myRole = ref<MyRole | null>(null);
const loaded = ref(false);
const loading = ref(false);
let lastLoadedGuild: string | null = null;

async function load(guildId: string) {
  if (loading.value || lastLoadedGuild === guildId) return;
  loading.value = true;
  try {
    const [list, me] = await Promise.all([
      rbacService.listComponentVisibility(guildId).catch(() => [] as ComponentVisibilityEntry[]),
      rbacService.getMyRole(guildId).catch(() => null),
    ]);
    overrides.value = list;
    myRole.value = me;
    lastLoadedGuild = guildId;
    loaded.value = true;
  } finally {
    loading.value = false;
  }
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

  // Charge a la 1ere utilisation + a chaque changement de guild.
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

  const isSuper = computed(() => myRole.value?.is_superadmin === true);
  const role = computed<RbacRole | null>(() => myRole.value?.role ?? null);

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
