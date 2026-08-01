// État partagé du back-office de la vie communautaire.
//
// Quatre entités distinctes (annonces de recherche, sondages, membre du mois,
// nouvelles) mais un seul écran à onglets : elles se pilotent ensemble et un
// modérateur passe de l'une à l'autre. Un composable par entité aurait
// multiplié par quatre le câblage `guilde sélectionnée → rechargement`.
//
// Portée module (singleton) comme les autres composables du back-office : le
// cache est partagé entre la page et ses organismes.

import { ref, watch } from "vue";

import { errMsg } from "@/utils/errMsg";
import { useGuildSelector } from "./useGuildSelector";
import { useToast } from "./useToast";
import {
  communityAdminService,
  type AdminLfgPost,
  type AdminNewsItem,
  type AdminPoll,
  type AdminSpotlight,
} from "@/services/communityAdminService";

export type LifeTab = "lfg" | "polls" | "spotlight" | "news";

const { selectedGuildId } = useGuildSelector();

const tab = ref<LifeTab>("news");
/// Inclure ce qui est clos, expiré ou en brouillon. Le back-office en a
/// besoin pour modérer ; la page publique ne le voit jamais.
const showArchived = ref(true);

const lfg = ref<AdminLfgPost[]>([]);
const polls = ref<AdminPoll[]>([]);
const spotlight = ref<AdminSpotlight[]>([]);
const news = ref<AdminNewsItem[]>([]);
const loading = ref(false);

async function fetchAll() {
  const { error: toastErr } = useToast();
  const guildId = selectedGuildId.value;
  if (!guildId) return;

  loading.value = true;
  try {
    // En parallèle : les quatre listes sont indépendantes, les enchaîner
    // quadruplerait le temps d'affichage de l'écran.
    const [l, p, s, n] = await Promise.all([
      communityAdminService.listLfg(guildId, showArchived.value),
      communityAdminService.listPolls(guildId, showArchived.value),
      communityAdminService.listSpotlight(guildId),
      communityAdminService.listNews(guildId, showArchived.value),
    ]);
    lfg.value = l;
    polls.value = p;
    spotlight.value = s;
    news.value = n;
  } catch (e: unknown) {
    toastErr(`Échec du chargement : ${errMsg(e)}`);
  } finally {
    loading.value = false;
  }
}

export function useCommunityLife() {
  return {
    tab,
    showArchived,
    lfg,
    polls,
    spotlight,
    news,
    loading,
    guildId: selectedGuildId,
    refresh: fetchAll,
  };
}

watch([selectedGuildId, showArchived], fetchAll, { immediate: true });
