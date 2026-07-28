import { computed } from "vue";
import type { DiscordRole } from "../types";
import { useGuildFetch } from "./useGuildFetch";
import { useSearch } from "./useSearch";
import { discordRolesService } from "@/services/discordRolesService";

// State module-scoped : un seul cache partage entre la page et tous
// les organisms (grid, modales create/edit). Sans ca, chaque appel
// useDiscordRoles() creerait son propre state et la search dans la page
// filtrerait un state different de celui affiche par la grid.
const { data: roles, loading, error, refresh: fetchRoles } = useGuildFetch<DiscordRole[]>(
  async (guildId) => guildId ? await discordRolesService.getAll(guildId) : [],
  [],
  { label: "roles Discord" },
);

const { search, filtered: filteredRoles } = useSearch<DiscordRole>(roles, ["name", "id"]);

const managedRoles = computed(() => roles.value.filter((r) => r.managed));
const customRoles = computed(() => roles.value.filter((r) => !r.managed));
const totalRoles = computed(() => roles.value.length);

export function useDiscordRoles() {
  return { roles, filteredRoles, managedRoles, customRoles, totalRoles, loading, error, search, fetchRoles };
}
