import { defineStore } from "pinia";
import { computed, ref } from "vue";
import type { MyRole, RbacRole } from "@/types";

/**
 * Store Pinia : identite applicative de l'utilisateur courant.
 *
 * Le back-office n'a plus qu'un mode d'acces — les Discord user IDs listes
 * dans SUPERADMIN_USER_IDS cote API. Quiconque atteint le front y figure
 * forcement : le middleware d'API refuse (403) tous les autres avant meme
 * d'atteindre un handler. Il n'y a donc plus de role a resoudre, ni d'appel
 * reseau a faire ici.
 *
 * Le store est conserve plutot que supprime avec ses sites d'appel : les
 * conditions d'interface qui l'interrogent restent valides, elles sont
 * simplement toujours vraies.
 */
const SUPERADMIN: MyRole = {
  discord_user_id: "",
  guild_id: "",
  role: "owner" as RbacRole,
  is_superadmin: true,
};

export const useMyRoleStore = defineStore("myRole", () => {
  const myRole = ref<MyRole | null>(SUPERADMIN);
  const loading = ref(false);

  const role = computed<RbacRole | null>(() => myRole.value?.role ?? null);
  const isSuper = computed(() => true);

  async function load(_guildId: string): Promise<MyRole | null> {
    return myRole.value;
  }

  function reset(): void {
    /* Rien a reinitialiser : l'identite ne depend plus de la guilde. */
  }

  function invalidate(): void {
    /* Rien a invalider : aucune donnee distante n'est mise en cache. */
  }

  return { myRole, role, isSuper, loading, load, reset, invalidate };
});
