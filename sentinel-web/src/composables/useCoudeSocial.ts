import { computed, ref } from "vue";
import { useGuildSelector } from "./useGuildSelector";
import { useToast } from "./useToast";
import { coudeSocialService } from "@/services/coudeSocialService";
import type { ActiveCurse, Prime } from "@/types/coude-social";

// Singleton module-scoped : un cache partage entre Lookup + cards.
const { guildIdFilter } = useGuildSelector();

const lookupUserId = ref("");
const loading = ref(false);

const curse = ref<ActiveCurse | null>(null);
const primes = ref<Prime[]>([]);

// Pot total = somme des primes non encore réclamées.
const bountyPot = computed(() =>
  primes.value.filter((p) => !p.claimed).reduce((sum, p) => sum + p.amount, 0),
);

async function lookup() {
  const { error: showError } = useToast();
  if (!guildIdFilter.value || !lookupUserId.value.trim()) {
    curse.value = null;
    primes.value = [];
    return;
  }
  const gid = guildIdFilter.value;
  const uid = lookupUserId.value.trim();
  loading.value = true;
  try {
    const [c, p] = await Promise.all([
      coudeSocialService.getActiveCurse(gid, uid).catch(() => null),
      coudeSocialService.listActivePrimes(gid, uid).catch(() => []),
    ]);
    curse.value = c;
    primes.value = p;
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
    curse, primes, bountyPot,
    lookup, liftCurse,
  };
}
