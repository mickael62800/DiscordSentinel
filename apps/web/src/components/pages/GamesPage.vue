<script setup lang="ts">
import { ref, computed } from "vue";
import { gamesService, type Game } from "@/services/gamesService";
import { useGames } from "@/composables/useGames";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useToast } from "@/composables/useToast";
import { useConfirm } from "@/composables/useConfirm";
import AppButton from "@/components/atoms/AppButton.vue";
import AppBadge from "@/components/atoms/AppBadge.vue";
import EmptyState from "@/components/atoms/EmptyState.vue";
import LoadingState from "@/components/atoms/LoadingState.vue";

const { selectedGuildId } = useGuildSelector();
const { games, panels, subscriberCounts, categories, loading, fetchAll } = useGames();
const { success: showSuccess, error: showError } = useToast();
const { confirm } = useConfirm();

// Filtre categorie
const selectedCategory = ref<string>("__all__");

const filteredGames = computed<Game[]>(() => {
  if (selectedCategory.value === "__all__") return games.value;
  if (selectedCategory.value === "__none__") {
    return games.value.filter((g) => !g.category || !g.category.trim());
  }
  return games.value.filter(
    (g) => (g.category ?? "").toLowerCase() === selectedCategory.value.toLowerCase(),
  );
});

// ── Preview emoji ──
// Extrait l'ID d'un emoji custom Discord `<:name:id>` ou `<a:name:id>`.
const customEmojiRe = /<(a?):([A-Za-z0-9_]+):(\d+)>/;
function emojiCdn(emoji: string | null): string | null {
  if (!emoji) return null;
  const m = emoji.match(customEmojiRe);
  if (!m) return null;
  const animated = m[1] === "a";
  const id = m[3];
  return `https://cdn.discordapp.com/emojis/${id}.${animated ? "gif" : "png"}?size=32`;
}
function emojiText(emoji: string | null): string {
  if (!emoji) return "";
  return customEmojiRe.test(emoji) ? "" : emoji;
}

// ── Modal Creer / Editer ──
interface ModalState {
  open: boolean;
  mode: "create" | "edit";
  gameId: string | null;
  name: string;
  category: string;
  imageFile: File | null;
  imagePreviewUrl: string | null;
  existingEmoji: string | null;
  submitting: boolean;
  error: string | null;
}

const modal = ref<ModalState>({
  open: false,
  mode: "create",
  gameId: null,
  name: "",
  category: "",
  imageFile: null,
  imagePreviewUrl: null,
  existingEmoji: null,
  submitting: false,
  error: null,
});

function openCreate() {
  modal.value = {
    open: true,
    mode: "create",
    gameId: null,
    name: "",
    category: selectedCategory.value !== "__all__" && selectedCategory.value !== "__none__"
      ? selectedCategory.value
      : "",
    imageFile: null,
    imagePreviewUrl: null,
    existingEmoji: null,
    submitting: false,
    error: null,
  };
}

function openEdit(game: Game) {
  modal.value = {
    open: true,
    mode: "edit",
    gameId: game.id,
    name: game.game_name,
    category: game.category ?? "",
    imageFile: null,
    imagePreviewUrl: null,
    existingEmoji: game.emoji,
    submitting: false,
    error: null,
  };
}

function closeModal() {
  if (modal.value.imagePreviewUrl) URL.revokeObjectURL(modal.value.imagePreviewUrl);
  modal.value.open = false;
}

function onFilePicked(ev: Event) {
  const input = ev.target as HTMLInputElement;
  const file = input.files?.[0];
  if (!file) return;
  setImageFile(file);
}

function setImageFile(file: File) {
  if (!/^image\/(png|jpe?g|gif|webp)$/i.test(file.type)) {
    modal.value.error = "Format non supporte (PNG/JPG/GIF/WEBP uniquement).";
    return;
  }
  if (file.size > 5 * 1024 * 1024) {
    modal.value.error = "Image trop grande (5 MB max avant resize).";
    return;
  }
  if (modal.value.imagePreviewUrl) URL.revokeObjectURL(modal.value.imagePreviewUrl);
  modal.value.imageFile = file;
  modal.value.imagePreviewUrl = URL.createObjectURL(file);
  modal.value.error = null;
}

// Drag & drop handlers
function onDragOver(ev: DragEvent) {
  ev.preventDefault();
}
function onDrop(ev: DragEvent) {
  ev.preventDefault();
  const file = ev.dataTransfer?.files?.[0];
  if (file) setImageFile(file);
}

/** Resize l'image a 128x128 PNG si necessaire. Retourne un Blob < 256 KB. */
async function resizeImage(file: File): Promise<Blob> {
  if (file.size <= 256 * 1024 && file.type === "image/png") {
    return file;
  }
  const bitmap = await createImageBitmap(file);
  const canvas = document.createElement("canvas");
  canvas.width = 128;
  canvas.height = 128;
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("Canvas indisponible");
  // Fit "contain" centre
  const scale = Math.min(128 / bitmap.width, 128 / bitmap.height);
  const w = bitmap.width * scale;
  const h = bitmap.height * scale;
  const x = (128 - w) / 2;
  const y = (128 - h) / 2;
  ctx.clearRect(0, 0, 128, 128);
  ctx.drawImage(bitmap, x, y, w, h);
  return await new Promise<Blob>((resolve, reject) => {
    canvas.toBlob(
      (b) => (b ? resolve(b) : reject(new Error("toBlob a echoue"))),
      "image/png",
      0.9,
    );
  });
}

async function submitModal() {
  const gid = selectedGuildId.value;
  if (!gid) {
    modal.value.error = "Aucun serveur selectionne.";
    return;
  }
  const name = modal.value.name.trim();
  if (!name || name.length > 64) {
    modal.value.error = "Nom invalide (1-64 caracteres).";
    return;
  }

  modal.value.submitting = true;
  modal.value.error = null;
  try {
    if (modal.value.mode === "create") {
      if (!modal.value.imageFile) {
        modal.value.error = "Image requise pour creer un jeu.";
        modal.value.submitting = false;
        return;
      }
      // Resize cote browser
      const blob = await resizeImage(modal.value.imageFile);
      if (blob.size > 256 * 1024) {
        modal.value.error = "Image toujours trop grosse apres resize.";
        modal.value.submitting = false;
        return;
      }
      const emojiResp = await gamesService.uploadEmoji(gid, name, blob);
      await gamesService.create({
        guild_id: gid,
        game_name: name,
        created_by: "web-ui",
        emoji: emojiResp.emoji,
        category: modal.value.category.trim() || null,
      });
      showSuccess(`Jeu "${name}" cree.`);
    } else if (modal.value.mode === "edit" && modal.value.gameId) {
      let newEmoji: string | undefined;
      if (modal.value.imageFile) {
        const blob = await resizeImage(modal.value.imageFile);
        if (blob.size > 256 * 1024) {
          modal.value.error = "Image trop grosse apres resize.";
          modal.value.submitting = false;
          return;
        }
        const emojiResp = await gamesService.uploadEmoji(gid, name, blob);
        newEmoji = emojiResp.emoji;
      }
      await gamesService.update(gid, modal.value.gameId, {
        game_name: name,
        category: modal.value.category.trim() || null,
        ...(newEmoji !== undefined ? { emoji: newEmoji } : {}),
      });
      showSuccess(`Jeu "${name}" mis a jour.`);
    }
    closeModal();
    await fetchAll();
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    modal.value.error = msg;
    showError(msg);
  } finally {
    modal.value.submitting = false;
  }
}

async function onDelete(game: Game) {
  const gid = selectedGuildId.value;
  if (!gid) return;
  const ok = await confirm({
    title: "Supprimer le jeu",
    message: `Supprimer "${game.game_name}" ? Tous les abonnements seront perdus.`,
  });
  if (!ok) return;
  try {
    await gamesService.delete(gid, game.id);
    showSuccess(`Jeu "${game.game_name}" supprime.`);
    await fetchAll();
  } catch (e) {
    showError(e instanceof Error ? e.message : String(e));
  }
}

// ── Panels ──
function jumpUrl(panel: { channel_id: string; message_id: string }): string {
  const gid = selectedGuildId.value ?? "@me";
  return `https://discord.com/channels/${gid}/${panel.channel_id}/${panel.message_id}`;
}
</script>

<template>
  <div class="games-page">
    <div class="page-header">
      <h1>Gestion des jeux</h1>
      <div class="header-actions">
        <select
          v-model="selectedCategory"
          class="category-select"
          :disabled="!selectedGuildId"
        >
          <option value="__all__">Toutes les categories</option>
          <option value="__none__">(Sans categorie)</option>
          <option v-for="c in categories" :key="c" :value="c">{{ c }}</option>
        </select>
        <AppButton variant="primary" :disabled="!selectedGuildId" @click="openCreate">
          + Ajouter un jeu
        </AppButton>
      </div>
    </div>

    <EmptyState
      v-if="!selectedGuildId"
      message="Selectionnez un serveur pour gerer les jeux."
    />
    <LoadingState v-else-if="loading" />
    <template v-else>
      <EmptyState
        v-if="filteredGames.length === 0"
        message="Aucun jeu dans cette categorie. Cliquez sur « Ajouter un jeu » pour commencer."
      />
      <div v-else class="games-table">
        <div class="row header-row">
          <div class="col emoji">Emoji</div>
          <div class="col name">Nom</div>
          <div class="col category">Categorie</div>
          <div class="col subs">Abonnes</div>
          <div class="col actions">Actions</div>
        </div>
        <div v-for="g in filteredGames" :key="g.id" class="row">
          <div class="col emoji">
            <img
              v-if="emojiCdn(g.emoji)"
              :src="emojiCdn(g.emoji)!"
              :alt="g.game_name"
              class="emoji-img"
            />
            <span v-else class="emoji-text">{{ emojiText(g.emoji) || "—" }}</span>
          </div>
          <div class="col name">{{ g.game_name }}</div>
          <div class="col category">
            <AppBadge
              v-if="g.category"
              :label="g.category"
              variant="info"
            />
            <span v-else class="muted">—</span>
          </div>
          <div class="col subs">{{ subscriberCounts[g.id] ?? 0 }}</div>
          <div class="col actions">
            <AppButton variant="secondary" size="sm" @click="openEdit(g)">Editer</AppButton>
            <AppButton variant="danger" size="sm" @click="onDelete(g)">Suppr.</AppButton>
          </div>
        </div>
      </div>

      <!-- Panels Discord -->
      <section class="panels-section">
        <h2>Panels Discord</h2>
        <p class="hint">
          Les panels sont deployes dans Discord via la commande
          <code>/game-admin panel category:&lt;nom&gt;</code>. Ils permettent aux membres de
          s'abonner aux jeux en cliquant sur des reactions.
        </p>
        <div v-if="panels.length === 0" class="muted">
          Aucun panel deploye. Utilisez la commande ci-dessus dans Discord.
        </div>
        <div v-else class="panels-list">
          <div v-for="p in panels" :key="p.id" class="panel-card">
            <div class="panel-head">
              <AppBadge
                :label="p.category ?? '(sans categorie)'"
                variant="info"
              />
              <a :href="jumpUrl(p)" target="_blank" rel="noopener" class="jump-link">
                Ouvrir dans Discord &rarr;
              </a>
            </div>
            <div class="panel-meta">
              <span>Salon : <code>{{ p.channel_id }}</code></span>
              <span>Message : <code>{{ p.message_id }}</code></span>
            </div>
          </div>
        </div>
      </section>
    </template>

    <!-- Modal -->
    <Teleport to="body">
      <div v-if="modal.open" class="modal-overlay" @click.self="closeModal">
        <div class="modal-dialog">
          <h3>{{ modal.mode === "create" ? "Ajouter un jeu" : "Editer le jeu" }}</h3>

          <label class="field">
            <span>Nom du jeu</span>
            <input
              v-model="modal.name"
              type="text"
              maxlength="64"
              placeholder="Ex : Valorant"
              class="input"
            />
          </label>

          <label class="field">
            <span>Categorie</span>
            <input
              v-model="modal.category"
              type="text"
              list="known-categories"
              placeholder="Ex : FPS"
              class="input"
            />
            <datalist id="known-categories">
              <option v-for="c in categories" :key="c" :value="c" />
            </datalist>
          </label>

          <div class="field">
            <span>Image de l'emoji</span>
            <div
              class="dropzone"
              :class="{ 'has-image': modal.imagePreviewUrl || modal.existingEmoji }"
              @dragover="onDragOver"
              @drop="onDrop"
            >
              <template v-if="modal.imagePreviewUrl">
                <img :src="modal.imagePreviewUrl" alt="preview" class="dropzone-preview" />
              </template>
              <template v-else-if="emojiCdn(modal.existingEmoji)">
                <img :src="emojiCdn(modal.existingEmoji)!" alt="emoji" class="dropzone-preview" />
                <span class="dropzone-hint">Emoji actuel — glissez une nouvelle image pour remplacer</span>
              </template>
              <template v-else>
                <span class="dropzone-hint">
                  Glissez une image (PNG/JPG/GIF) ou
                </span>
              </template>
              <label class="browse-btn">
                Parcourir
                <input
                  type="file"
                  accept="image/png,image/jpeg,image/gif,image/webp"
                  @change="onFilePicked"
                  hidden
                />
              </label>
            </div>
            <p class="sub-hint">
              Resize automatique a 128x128 PNG cote navigateur. Max 256 KB apres resize.
            </p>
          </div>

          <p v-if="modal.error" class="error-msg">{{ modal.error }}</p>

          <div class="modal-actions">
            <AppButton variant="secondary" :disabled="modal.submitting" @click="closeModal">
              Annuler
            </AppButton>
            <AppButton variant="primary" :disabled="modal.submitting" @click="submitModal">
              {{ modal.submitting ? "..." : modal.mode === "create" ? "Creer" : "Enregistrer" }}
            </AppButton>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.games-page { padding: 4px 0; }

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
  gap: 12px;
  flex-wrap: wrap;
}

.header-actions {
  display: flex;
  gap: 10px;
  align-items: center;
}

.category-select {
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg-primary);
  color: var(--text-primary);
  font-size: 13px;
  min-width: 200px;
}

.games-table {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  overflow: hidden;
}

.row {
  display: grid;
  grid-template-columns: 80px 1fr 200px 100px 180px;
  gap: 12px;
  padding: 12px 16px;
  border-bottom: 1px solid var(--border);
  align-items: center;
}
.row:last-child { border-bottom: none; }
.header-row {
  background: var(--bg-secondary);
  font-size: 11px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--text-secondary);
}

.col.emoji { display: flex; align-items: center; justify-content: center; }
.emoji-img { width: 28px; height: 28px; object-fit: contain; }
.emoji-text { font-size: 20px; }

.col.name { font-weight: 600; color: var(--text-primary); }
.col.subs { font-variant-numeric: tabular-nums; }
.col.actions { display: flex; gap: 6px; justify-content: flex-end; }

.muted { color: var(--text-secondary); }

/* Panels */
.panels-section { margin-top: 32px; }
.panels-section h2 { font-size: 15px; font-weight: 600; margin-bottom: 10px; }
.hint { font-size: 13px; color: var(--text-secondary); margin-bottom: 14px; }
.hint code {
  background: var(--bg-secondary);
  padding: 2px 6px;
  border-radius: 4px;
  font-family: "JetBrains Mono", monospace;
  font-size: 12px;
}

.panels-list { display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: 12px; }
.panel-card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 14px;
}
.panel-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}
.jump-link {
  font-size: 12px;
  color: var(--accent);
  text-decoration: none;
}
.jump-link:hover { text-decoration: underline; }
.panel-meta { display: flex; flex-direction: column; gap: 4px; font-size: 11px; color: var(--text-secondary); }
.panel-meta code { font-family: "JetBrains Mono", monospace; }

/* Modal */
.modal-overlay {
  position: fixed; inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex; align-items: center; justify-content: center;
  z-index: 9999;
}
.modal-dialog {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 24px;
  width: 480px;
  max-width: 90vw;
  max-height: 90vh;
  overflow-y: auto;
}
.modal-dialog h3 { font-size: 17px; font-weight: 600; margin-bottom: 18px; }

.field { display: flex; flex-direction: column; gap: 6px; margin-bottom: 14px; }
.field > span { font-size: 13px; font-weight: 500; color: var(--text-primary); }
.input {
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--bg-primary);
  color: var(--text-primary);
  font-size: 13px;
}

.dropzone {
  border: 2px dashed var(--border);
  border-radius: 10px;
  padding: 20px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
  background: var(--bg-secondary);
  min-height: 140px;
  justify-content: center;
}
.dropzone.has-image { border-style: solid; }
.dropzone-preview {
  width: 96px; height: 96px;
  object-fit: contain;
  background: var(--bg-primary);
  border-radius: 8px;
  border: 1px solid var(--border);
}
.dropzone-hint { font-size: 12px; color: var(--text-secondary); text-align: center; }
.browse-btn {
  padding: 6px 14px;
  border: 1px solid var(--border);
  border-radius: 6px;
  font-size: 12px;
  cursor: pointer;
  background: var(--bg-primary);
  color: var(--text-primary);
}
.browse-btn:hover { border-color: var(--accent); color: var(--accent); }

.sub-hint { font-size: 11px; color: var(--text-secondary); margin-top: 6px; }

.error-msg {
  padding: 10px;
  border-radius: 6px;
  background: var(--danger-bg, rgba(239, 68, 68, 0.15));
  color: var(--danger);
  font-size: 13px;
  margin-bottom: 14px;
}

.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  margin-top: 8px;
}

@media (max-width: 900px) {
  .row { grid-template-columns: 60px 1fr 1fr; }
  .row .col.subs, .row .col.actions { grid-column: 1 / -1; }
  .row .col.actions { justify-content: flex-start; }
}
</style>
