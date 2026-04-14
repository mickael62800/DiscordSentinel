import { computed } from "vue";
import type { DiscordRole } from "../types";
import { useGuildFetch } from "./useGuildFetch";
import { useSearch } from "./useSearch";
import { discordRolesService } from "@/services/discordRolesService";

export function useDiscordRoles() {
  const { data: roles, loading, error, refresh: fetchRoles } = useGuildFetch<DiscordRole[]>(
    async (guildId) => guildId ? await discordRolesService.getAll(guildId) : [],
    [],
    { label: "roles Discord" },
  );

  const { search, filtered: filteredRoles } = useSearch<DiscordRole>(roles, ["name", "id"]);

  const managedRoles = computed(() => roles.value.filter((r) => r.managed));
  const customRoles = computed(() => roles.value.filter((r) => !r.managed));
  const totalRoles = computed(() => roles.value.length);

  return { roles, filteredRoles, managedRoles, customRoles, totalRoles, loading, error, search, fetchRoles };
}
