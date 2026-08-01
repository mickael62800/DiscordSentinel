<script setup lang="ts">
// Les jeux de la communauté, jouables depuis le site.
//
// # Un seul portefeuille
//
// Rien n'est calculé ici. Le tirage, le quota quotidien et les mouvements de
// coins vivent dans nexus-core, appelé par le même chemin que le bot Discord.
// Le solde affiché EST celui de Discord — pas une copie synchronisée, le même.
// Avoir déjà tiré sur Discord fait échouer le tirage ici, et réciproquement.
//
// # L'animation
//
// La roue tourne pendant que la requête part, puis s'arrête sur la case que
// le serveur a réellement tirée. L'ordre compte : faire tourner puis demander
// le résultat laisserait croire que l'animation le détermine. Ici elle ne fait
// que le mettre en scène.

import { computed, onMounted, ref } from "vue";

import { useAuth } from "../../composables/useAuth";
import { COMMUNITY, onWordmarkError, wordmarkOf } from "@/branding";
import {
  gamesService,
  type Rank,
  type SpinResult,
  type Transaction,
  type Wallet,
} from "@/services/gamesService";

const { user } = useAuth();

const wallet = ref<Wallet | null>(null);
const history = ref<Transaction[]>([]);
const ranking = ref<Rank[]>([]);
const chargement = ref(true);
const indisponible = ref(false);

// ── Roue ──

/// Les dix cases, dans l'ordre où elles sont dessinées. Les libellés viennent
/// du serveur à chaque tirage ; cette liste ne sert qu'au rendu de la roue au
/// repos, et les clés doivent correspondre à celles de `wheel.rs`.
const CASES = [
  { key: "blanche", court: "Rien", emoji: "🌀", couleur: "#6b7280" },
  { key: "pq", court: "+50", emoji: "🧻", couleur: "#94a3b8" },
  { key: "sieste", court: "+200", emoji: "💤", couleur: "#38bdf8" },
  { key: "colis", court: "+500", emoji: "📦", couleur: "#22c55e" },
  { key: "trefle", court: "+1000", emoji: "🍀", couleur: "#16a34a" },
  { key: "couronne", court: "+1500", emoji: "👑", couleur: "#f39c12" },
  { key: "ruine", court: "-500", emoji: "💀", couleur: "#f43f5e" },
  { key: "jackpot", court: "+5000", emoji: "🎰", couleur: "#a855f7" },
  { key: "bombe", court: "-2000", emoji: "💣", couleur: "#dc2626" },
  { key: "licorne", court: "+10000", emoji: "🦄", couleur: "#e879f9" },
];

const SECTEUR = 360 / CASES.length;

const enCours = ref(false);
const resultat = ref<SpinResult | null>(null);
const erreurRoue = ref<string | null>(null);
/// Angle cumulé, jamais remis à zéro : revenir en arrière ferait tourner la
/// roue à l'envers entre deux tirages.
const angle = ref(0);

async function tirer() {
  if (enCours.value || !user.value) return;
  enCours.value = true;
  erreurRoue.value = null;
  resultat.value = null;

  // Quelques tours pleins avant même de connaître l'issue : l'attente fait
  // partie du jeu, et la requête se déroule pendant ce temps.
  angle.value += 360 * 4;

  try {
    const r = await gamesService.spinWheel();

    const index = Math.max(0, CASES.findIndex((c) => c.key === r.case_key));
    // On complète jusqu'au secteur voulu, en restant dans le même sens.
    const vise = 360 - index * SECTEUR - SECTEUR / 2;
    const restant = (vise - (angle.value % 360) + 360) % 360;
    angle.value += restant;

    // Laisse l'animation finir avant d'annoncer : lire le gain pendant que la
    // roue tourne encore gâche le seul moment de suspense du jeu.
    await new Promise((r) => setTimeout(r, 3200));

    resultat.value = r;
    if (wallet.value) wallet.value.coins = r.balance_after;
    // Le tirage vient de créer une transaction : on recharge plutôt que de
    // la fabriquer côté client, où elle divergerait du libellé serveur.
    history.value = await gamesService.history();
    ranking.value = await gamesService.leaderboard();
  } catch (e) {
    erreurRoue.value = e instanceof Error ? e.message : "Le tirage a échoué.";
  } finally {
    enCours.value = false;
  }
}

// ── Chargement ──

onMounted(async () => {
  if (!user.value) {
    chargement.value = false;
    return;
  }
  try {
    const [w, h, l] = await Promise.all([
      gamesService.wallet(),
      gamesService.history(),
      gamesService.leaderboard(),
    ]);
    wallet.value = w;
    history.value = h;
    ranking.value = l;
  } catch {
    // Plateforme jeux éteinte ou non configurée : on le dit, plutôt que
    // d'afficher un portefeuille vide qui ferait croire à une perte de coins.
    indisponible.value = true;
  } finally {
    chargement.value = false;
  }
});

// ── Affichage ──

const solde = computed(() => wallet.value?.coins ?? 0);

function fmtCoins(n: number): string {
  return n.toLocaleString("fr-FR");
}

function fmtDate(iso: string): string {
  return new Date(iso).toLocaleString("fr-FR", {
    day: "numeric",
    month: "short",
    hour: "2-digit",
    minute: "2-digit",
  });
}

/// Icône selon l'origine du mouvement. Le libellé technique du serveur ne
/// doit pas remonter tel quel à l'écran.
function icone(source: string): string {
  if (source.startsWith("wheel")) return "🎡";
  if (source.includes("transfer")) return "🤝";
  if (source.includes("coude")) return "💥";
  return "🪙";
}

/// Dégradé conique dessinant les dix secteurs. Calculé une fois : le
/// recalculer à chaque rendu ferait clignoter la roue pendant sa rotation.
const fondRoue = computed(() => {
  const parts = CASES.map((c, i) => {
    const de = i * SECTEUR;
    const a = (i + 1) * SECTEUR;
    return `${c.couleur} ${de}deg ${a}deg`;
  });
  return `conic-gradient(${parts.join(", ")})`;
});
</script>

<template>
  <div class="jx">
    <header class="jx-bar">
      <RouterLink to="/membre" class="jx-ghost">← L'espace membre</RouterLink>
      <span v-if="user" class="jx-solde" :class="{ pulse: !!resultat }">
        🪙 {{ fmtCoins(solde) }}
      </span>
    </header>

    <section class="jx-hero">
      <img
        class="jx-logo"
        :src="wordmarkOf(COMMUNITY)"
        :alt="COMMUNITY.name"
        @error="onWordmarkError($event, COMMUNITY)"
      />
      <h1>Les jeux du canapé</h1>
      <p>
        Le même porte-monnaie que sur Discord. Ce que tu gagnes ici, tu le
        retrouves là-bas.
      </p>
    </section>

    <!-- Non connecté : on montre le jeu, on demande la connexion pour agir. -->
    <section v-if="!user" class="jx-block">
      <p class="jx-vide">
        Connecte-toi pour tirer la Roue et retrouver ton porte-monnaie.
      </p>
      <RouterLink to="/login?espace=membre" class="jx-cta">Se connecter</RouterLink>
    </section>

    <p v-else-if="chargement" class="jx-hint">Chargement…</p>

    <section v-else-if="indisponible" class="jx-block">
      <p class="jx-alerte">
        La plateforme de jeux ne répond pas. Ton porte-monnaie n'est pas perdu,
        il est simplement inaccessible pour l'instant.
      </p>
    </section>

    <template v-else>
      <!-- ── La Roue ── -->
      <section class="jx-block">
        <h2>La Roue du Destin <span class="jx-count">un tirage par jour</span></h2>

        <div class="jx-roue-zone">
          <div class="jx-roue-wrap">
            <span class="jx-fleche" aria-hidden="true"></span>
            <div
              class="jx-roue"
              :style="{
                background: fondRoue,
                transform: `rotate(${angle}deg)`,
              }"
            >
              <span
                v-for="(c, i) in CASES"
                :key="c.key"
                class="jx-case"
                :style="{ transform: `rotate(${i * SECTEUR + SECTEUR / 2}deg)` }"
              >
                <span class="jx-case-in">{{ c.emoji }}</span>
              </span>
            </div>
          </div>

          <div class="jx-roue-cote">
            <button
              type="button"
              class="jx-cta grand"
              :disabled="enCours"
              @click="tirer"
            >
              {{ enCours ? "Ça tourne…" : "Tirer la Roue" }}
            </button>

            <p v-if="erreurRoue" class="jx-alerte">{{ erreurRoue }}</p>

            <div v-else-if="resultat" class="jx-resultat" :class="{ rare: resultat.is_memorable }">
              <strong>{{ resultat.case_label }}</strong>
              <span
                v-if="resultat.payout !== 0"
                class="jx-gain"
                :class="resultat.payout > 0 ? 'plus' : 'moins'"
              >
                {{ resultat.payout > 0 ? "+" : "" }}{{ fmtCoins(resultat.payout) }} coins
              </span>
              <span v-else class="jx-gain neutre">Rien. Du tout.</span>
              <span class="jx-apres">Nouveau solde : {{ fmtCoins(resultat.balance_after) }}</span>
            </div>

            <p v-else class="jx-vide">
              Dix cases, de la ruine à la licorne. Le tirage est le même que
              celui de <code>/roue</code> sur Discord.
            </p>
          </div>
        </div>
      </section>

      <!-- ── Classement ── -->
      <section class="jx-block">
        <h2>Les plus riches</h2>

        <p v-if="!ranking.length" class="jx-vide">Personne n'a encore de coins.</p>

        <ol v-else class="jx-rangs">
          <li v-for="r in ranking" :key="r.rank" class="jx-rang" :class="{ moi: r.is_me }">
            <span class="jx-place">{{ r.rank }}</span>
            <span class="jx-nom">{{ r.username || "Un membre" }}</span>
            <span class="jx-coins">{{ fmtCoins(r.coins) }}</span>
          </li>
        </ol>
      </section>

      <!-- ── Historique ── -->
      <section class="jx-block">
        <h2>Tes derniers mouvements</h2>

        <p v-if="!history.length" class="jx-vide">
          Aucun mouvement. Ton premier tirage apparaîtra ici.
        </p>

        <ul v-else class="jx-txs">
          <li v-for="t in history" :key="t.id" class="jx-tx">
            <span class="jx-tx-ico" aria-hidden="true">{{ icone(t.source) }}</span>
            <span class="jx-tx-desc">{{ t.description }}</span>
            <span class="jx-tx-montant" :class="t.amount >= 0 ? 'plus' : 'moins'">
              {{ t.amount > 0 ? "+" : "" }}{{ fmtCoins(t.amount) }}
            </span>
            <span class="jx-tx-date">{{ fmtDate(t.created_at) }}</span>
          </li>
        </ul>
      </section>
    </template>
  </div>
</template>

<style scoped>
.jx {
  --surface: rgba(255, 255, 255, 0.045);
  --line: rgba(168, 85, 247, 0.22);
  --line-strong: rgba(168, 85, 247, 0.5);
  --accent: #a855f7;
  --ink: #f3eaff;
  --ink-2: #d8c7f5;
  --ink-3: #c3aee6;
  --ink-4: #b49ad8;
  --plus: #22c55e;
  --moins: #f43f5e;

  flex: 1;
  overflow-x: hidden;
  overflow-y: auto;
  padding: clamp(1rem, 3vh, 2rem) clamp(1rem, 4vw, 3rem) 3rem;
  background: linear-gradient(180deg, #150a28 0%, #0d0619 55%, #08040f 100%);
  color: var(--ink);
  display: flex;
  flex-direction: column;
  gap: clamp(1.75rem, 4vh, 2.5rem);
}

.jx-bar,
.jx-hero,
.jx-block {
  width: 100%;
  max-width: 62rem;
  margin: 0 auto;
}

.jx-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}

.jx-ghost {
  border: 1px solid var(--line-strong);
  color: var(--ink-2);
  border-radius: 999px;
  padding: 0.3rem 0.95rem;
  font-size: 0.88rem;
  text-decoration: none;
}

.jx-ghost:hover {
  border-color: var(--accent);
  color: #fff;
}

.jx-solde {
  font-variant-numeric: tabular-nums;
  font-weight: 700;
  font-size: 1.05rem;
  padding: 0.3rem 1rem;
  border-radius: 999px;
  background: var(--surface);
  border: 1px solid var(--line);
}

/* Le solde change à l'issue du tirage : un bref éclat signale la mise à
   jour, sinon le chiffre bouge sans qu'on le remarque. */
.jx-solde.pulse {
  animation: eclat 0.9s ease-out;
}

@keyframes eclat {
  40% {
    border-color: var(--accent);
    color: #fff;
    transform: scale(1.06);
  }
}

.jx-hero {
  text-align: center;
}

.jx-logo {
  display: block;
  margin: 0 auto 0.75rem;
  width: min(180px, 40vw);
  height: auto;
  filter: drop-shadow(0 8px 30px rgba(168, 85, 247, 0.3));
}

.jx-hero h1 {
  margin: 0 0 0.3rem;
  font-size: clamp(1.4rem, 4vh, 2rem);
}

.jx-hero p {
  margin: 0;
  color: var(--ink-2);
}

.jx-block h2 {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  margin: 0 0 1rem;
  font-size: 1.15rem;
}

.jx-count {
  font-size: 0.8rem;
  font-weight: 400;
  color: var(--ink-4);
}

.jx-hint {
  text-align: center;
  color: var(--ink-4);
}

.jx-vide {
  margin: 0;
  padding: 0.85rem 1.05rem;
  border-radius: 0.9rem;
  background: rgba(255, 255, 255, 0.025);
  border: 1px dashed var(--line);
  color: var(--ink-4);
  font-size: 0.9rem;
  line-height: 1.5;
}

.jx-vide code {
  font-family: ui-monospace, "Cascadia Mono", Menlo, monospace;
  color: var(--ink-2);
}

.jx-alerte {
  margin: 0;
  padding: 0.85rem 1.05rem;
  border-radius: 0.9rem;
  background: rgba(244, 63, 94, 0.1);
  border: 1px solid rgba(244, 63, 94, 0.35);
  color: #fca5a5;
  font-size: 0.9rem;
}

.jx-cta {
  display: inline-block;
  align-self: flex-start;
  margin-top: 0.8rem;
  background: linear-gradient(135deg, var(--accent), #7c3aed);
  color: #fff;
  font: inherit;
  font-weight: 600;
  font-size: 0.9rem;
  border: none;
  border-radius: 999px;
  padding: 0.5rem 1.4rem;
  cursor: pointer;
  text-decoration: none;
}

.jx-cta.grand {
  font-size: 1rem;
  padding: 0.7rem 2rem;
  margin-top: 0;
}

.jx-cta:disabled {
  opacity: 0.55;
  cursor: default;
}

/* ── La roue ── */
.jx-roue-zone {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  gap: 2rem;
  align-items: center;
}

.jx-roue-wrap {
  position: relative;
  width: min(20rem, 60vw);
  aspect-ratio: 1;
}

/* Repère fixe en haut : c'est lui qui désigne la case gagnante. */
.jx-fleche {
  position: absolute;
  top: -0.6rem;
  left: 50%;
  translate: -50% 0;
  z-index: 2;
  width: 0;
  height: 0;
  border-left: 0.7rem solid transparent;
  border-right: 0.7rem solid transparent;
  border-top: 1.1rem solid var(--ink);
  filter: drop-shadow(0 2px 4px rgba(0, 0, 0, 0.6));
}

.jx-roue {
  position: absolute;
  inset: 0;
  border-radius: 50%;
  border: 4px solid rgba(255, 255, 255, 0.12);
  box-shadow: 0 12px 50px rgba(168, 85, 247, 0.25);
  /* Décélération longue : l'essentiel du plaisir est dans le ralentissement. */
  transition: transform 3s cubic-bezier(0.16, 1, 0.3, 1);
}

.jx-case {
  position: absolute;
  inset: 0;
  display: flex;
  justify-content: center;
  /* Chaque emoji est poussé vers le bord puis redressé, sinon il pencherait
     avec son secteur. */
  padding-top: 0.9rem;
}

.jx-case-in {
  font-size: 1.5rem;
  line-height: 1;
}

.jx-roue-cote {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  align-items: flex-start;
}

.jx-resultat {
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
  padding: 1rem 1.2rem;
  border-radius: 0.9rem;
  background: var(--surface);
  border: 1px solid var(--line);
}

.jx-resultat.rare {
  border-color: var(--accent);
  box-shadow: 0 0 30px rgba(168, 85, 247, 0.35);
}

.jx-gain {
  font-size: 1.3rem;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
}

.jx-gain.plus {
  color: var(--plus);
}

.jx-gain.moins {
  color: var(--moins);
}

.jx-gain.neutre {
  color: var(--ink-4);
  font-size: 1rem;
}

.jx-apres {
  font-size: 0.85rem;
  color: var(--ink-3);
}

/* ── Classement ── */
.jx-rangs {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}

.jx-rang {
  display: grid;
  grid-template-columns: 2.2rem 1fr auto;
  align-items: center;
  gap: 0.8rem;
  padding: 0.6rem 1rem;
  border-radius: 0.75rem;
  background: var(--surface);
  border: 1px solid var(--line);
}

.jx-rang.moi {
  border-color: var(--line-strong);
  background: rgba(168, 85, 247, 0.1);
}

.jx-place {
  font-variant-numeric: tabular-nums;
  font-weight: 700;
  color: var(--ink-4);
  text-align: right;
}

.jx-rang.moi .jx-place {
  color: var(--accent);
}

.jx-nom {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.jx-coins {
  font-variant-numeric: tabular-nums;
  font-weight: 600;
}

/* ── Historique ── */
.jx-txs {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}

.jx-tx {
  display: grid;
  grid-template-columns: auto 1fr auto auto;
  align-items: center;
  gap: 0.8rem;
  padding: 0.55rem 1rem;
  border-radius: 0.75rem;
  background: var(--surface);
  border: 1px solid var(--line);
  font-size: 0.9rem;
}

.jx-tx-desc {
  color: var(--ink-3);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.jx-tx-montant {
  font-variant-numeric: tabular-nums;
  font-weight: 700;
}

.jx-tx-montant.plus {
  color: var(--plus);
}

.jx-tx-montant.moins {
  color: var(--moins);
}

.jx-tx-date {
  font-size: 0.78rem;
  color: var(--ink-4);
  white-space: nowrap;
}

@media (max-width: 760px) {
  .jx-roue-zone {
    grid-template-columns: 1fr;
    justify-items: center;
  }

  .jx-roue-cote {
    align-items: center;
    text-align: center;
  }

  .jx-tx {
    grid-template-columns: auto 1fr auto;
  }

  .jx-tx-date {
    display: none;
  }
}

@media (prefers-reduced-motion: reduce) {
  .jx-roue {
    transition: none;
  }

  .jx-solde.pulse {
    animation: none;
  }
}
</style>
