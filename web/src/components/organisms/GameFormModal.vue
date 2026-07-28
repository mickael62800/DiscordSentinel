<script setup lang="ts">
import { ref, watch } from "vue";
import { errMsg } from "@/utils/errMsg";
import { gamesService, type Game } from "@/services/gamesService";
import { useGames } from "@/composables/useGames";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useToast } from "@/composables/useToast";
import AppButton from "@/components/atoms/AppButton.vue";
import AppModal from "@/components/atoms/AppModal.vue";

const props = defineProps<{
  visible: boolean;
  /** null = mode "create", sinon mode "edit" */
  target: Game | null;
  /** Categorie pre-selectionnee (mode create), si filtree dans la liste. */
  defaultCategory?: string;
}>();

const emit = defineEmits<{ close: [] }>();

const { selectedGuildId } = useGuildSelector();
const { categories, fetchAll } = useGames();
const { success: showSuccess, error: showError } = useToast();

interface FormState {
  name: string;
  category: string;
  imageFile: File | null;
  imagePreviewUrl: string | null;
  existingEmoji: string | null;
  submitting: boolean;
  error: string | null;
}

function emptyForm(): FormState {
  return {
    name: "",
    category: "",
    imageFile: null,
    imagePreviewUrl: null,
    existingEmoji: null,
    submitting: false,
    error: null,
  };
}

const form = ref<FormState>(emptyForm());

const customEmojiRe = /<(a?):([A-Za-z0-9_]+):(\d+)>/;
function emojiCdn(emoji: string | null): string | null {
  if (!emoji) return null;
  const m = emoji.match(customEmojiRe);
  if (!m) return null;
  const animated = m[1] === "a";
  const id = m[3];
  return `https://cdn.discordapp.com/emojis/${id}.${animated ? "gif" : "png"}?size=32`;
}

watch(
  () => props.visible,
  (v) => {
    if (!v) {
      // Cleanup preview URL au close
      if (form.value.imagePreviewUrl) URL.revokeObjectURL(form.value.imagePreviewUrl);
      return;
    }
    if (props.target) {
      form.value = {
        name: props.target.game_name,
        category: props.target.category ?? "",
        imageFile: null,
        imagePreviewUrl: null,
        existingEmoji: props.target.emoji,
        submitting: false,
        error: null,
      };
    } else {
      form.value = { ...emptyForm(), category: props.defaultCategory ?? "" };
    }
  },
);

function onFilePicked(ev: Event) {
  const input = ev.target as HTMLInputElement;
  const file = input.files?.[0];
  if (!file) return;
  setImageFile(file);
}

function setImageFile(file: File) {
  if (!/^image\/(png|jpe?g|gif|webp)$/i.test(file.type)) {
    form.value.error = "Format non supporte (PNG/JPG/GIF/WEBP uniquement).";
    return;
  }
  if (file.size > 5 * 1024 * 1024) {
    form.value.error = "Image trop grande (5 MB max avant resize).";
    return;
  }
  if (form.value.imagePreviewUrl) URL.revokeObjectURL(form.value.imagePreviewUrl);
  form.value.imageFile = file;
  form.value.imagePreviewUrl = URL.createObjectURL(file);
  form.value.error = null;
}

function onDragOver(ev: DragEvent) { ev.preventDefault(); }
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

async function submit() {
  const gid = selectedGuildId.value;
  if (!gid) {
    form.value.error = "Aucun serveur selectionne.";
    return;
  }
  const name = form.value.name.trim();
  if (!name || name.length > 64) {
    form.value.error = "Nom invalide (1-64 caracteres).";
    return;
  }

  form.value.submitting = true;
  form.value.error = null;
  try {
    const isCreate = !props.target;
    if (isCreate) {
      // Image optionnelle : si fournie, on upload l'emoji sur Discord.
      // Sinon le jeu est cree sans emoji (le panel utilisera juste le nom).
      let emoji: string | null = null;
      if (form.value.imageFile) {
        const blob = await resizeImage(form.value.imageFile);
        if (blob.size > 256 * 1024) {
          form.value.error = "Image toujours trop grosse apres resize.";
          form.value.submitting = false;
          return;
        }
        const emojiResp = await gamesService.uploadEmoji(gid, name, blob);
        emoji = emojiResp.emoji;
      }
      await gamesService.create({
        guild_id: gid,
        game_name: name,
        created_by: "web-ui",
        emoji,
        category: form.value.category.trim() || null,
      });
      showSuccess(`Jeu "${name}" cree.`);
    } else if (props.target) {
      let newEmoji: string | undefined;
      if (form.value.imageFile) {
        const blob = await resizeImage(form.value.imageFile);
        if (blob.size > 256 * 1024) {
          form.value.error = "Image trop grosse apres resize.";
          form.value.submitting = false;
          return;
        }
        const emojiResp = await gamesService.uploadEmoji(gid, name, blob);
        newEmoji = emojiResp.emoji;
      }
      await gamesService.update(gid, props.target.id, {
        game_name: name,
        category: form.value.category.trim() || null,
        ...(newEmoji !== undefined ? { emoji: newEmoji } : {}),
      });
      showSuccess(`Jeu "${name}" mis a jour.`);
    }
    emit("close");
    await fetchAll();
  } catch (e) {
    const msg = errMsg(e);
    form.value.error = msg;
    showError(msg);
  } finally {
    form.value.submitting = false;
  }
}
</script>

<template>
  <AppModal
    :visible="visible"
    :title="target ? 'Editer le jeu' : 'Ajouter un jeu'"
    size="md"
    @close="emit('close')"
  >
    <label class="field">
      <span>Nom du jeu</span>
      <input
        v-model="form.name"
        type="text"
        maxlength="64"
        placeholder="Ex : Valorant"
        class="input"
      />
    </label>

    <label class="field">
      <span>Categorie</span>
      <input
        v-model="form.category"
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
      <span>Image de l'emoji <em class="optional">(optionnel)</em></span>
      <div
        class="dropzone"
        :class="{ 'has-image': form.imagePreviewUrl || form.existingEmoji }"
        @dragover="onDragOver"
        @drop="onDrop"
      >
        <template v-if="form.imagePreviewUrl">
          <img :src="form.imagePreviewUrl" alt="preview" class="dropzone-preview" />
        </template>
        <template v-else-if="emojiCdn(form.existingEmoji)">
          <img :src="emojiCdn(form.existingEmoji)!" alt="emoji" class="dropzone-preview" />
          <span class="dropzone-hint">Emoji actuel — glissez une nouvelle image pour remplacer</span>
        </template>
        <template v-else>
          <span class="dropzone-hint">Glissez une image (PNG/JPG/GIF) ou</span>
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
        Recommande — sur le panel Discord, l'emoji devient l'icone du
        bouton du jeu (les membres cliquent dessus pour s'abonner). Sans
        emoji, le bouton affiche le nom du jeu. Resize automatique a 128x128 PNG.
      </p>
    </div>

    <p v-if="form.error" class="error-msg">{{ form.error }}</p>

    <template #footer>
      <AppButton variant="secondary" :disabled="form.submitting" @click="emit('close')">
        Annuler
      </AppButton>
      <AppButton variant="primary" :disabled="form.submitting" @click="submit">
        {{ form.submitting ? "..." : target ? "Enregistrer" : "Creer" }}
      </AppButton>
    </template>
  </AppModal>
</template>

<style scoped>
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

.optional { color: var(--text-secondary); font-style: italic; font-weight: 400; font-size: 12px; }

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
</style>
