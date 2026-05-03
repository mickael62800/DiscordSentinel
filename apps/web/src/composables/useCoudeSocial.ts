import { ref } from "vue";
import { useGuildSelector } from "./useGuildSelector";
import { useToast } from "./useToast";
import { coudeSocialService } from "@/services/coudeSocialService";
import type {
  ActiveBounty,
  ActiveCoalition,
  ActiveCurse,
  ActiveVendetta,
} from "@/types/coude-social";

// Singleton module-scoped : un cache partage entre Lookup + 4 cards.
const { guildIdFilter } = useGuildSelector();

const lookupUserId = ref("");
const loading = ref(false);

const curse = ref<ActiveCurse | null>(null);
const bounty = ref<ActiveBounty | null>(null);
const coalition = ref<ActiveCoalition | null>(null);
const vendettasAsChallenger = ref<ActiveVendetta[]>([]);

async function lookup() {
  const { error: showError } = useToast();
  if (!guildIdFilter.value || !lookupUserId.value.trim()) {
    curse.value = null;
    bounty.value = null;
    coalition.value = null;
    vendettasAsChallenger.value = [];
    return;
  }
  const gid = guildIdFilter.value;
  const uid = lookupUserId.value.trim();
  loading.value = true;
  try {
    const [c, b, co, v] = await Promise.all([
      coudeSocialService.getActiveCurse(gid, uid).catch(() => null),
      coudeSocialService.getBountyByTarget(gid, uid).catch(() => null),
      coudeSocialService.getCoalitionByTarget(gid, uid).catch(() => null),
      coudeSocialService.listVendettasByChallenger(gid, uid).catch(() => []),
    ]);
    curse.value = c;
    bounty.value = b;
    coalition.value = co;
    vendettasAsChallenger.value = v;
  } catch (e) {
    console.error("Erreur lookup coude social :", e);
    showError("Erreur de chargement.");
  } finally {
    loading.value = false;
  }
}

export function useCoudeSocial() {
  const { success, error: showError } = useToast();

  async function liftCurse() {
    if (!guildIdFilter.value || !lookupUserId.value.trim() || !curse.value) return;
    if (!confirm("Lever cette malédiction (admin override) ?")) return;
    try {
      await coudeSocialService.liftCurse(
        guildIdFilter.value,
        lookupUserId.value.trim(),
        "desktop",
      );
      curse.value = null;
      success("Malédiction levée.");
    } catch (e) {
      console.error(e);
      showError("Erreur lors de la levée.");
    }
  }

  return {
    lookupUserId, loading,
    curse, bounty, coalition, vendettasAsChallenger,
    lookup, liftCurse,
  };
}
