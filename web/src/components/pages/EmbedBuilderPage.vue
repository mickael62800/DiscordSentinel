<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { embedsService, type Embed, type EmbedInput } from "@/services/embedsService";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useToast } from "@/composables/useToast";
import { useConfirm } from "@/composables/useConfirm";
import { errMsg } from "@/utils/errMsg";
import { renderDiscordMarkdown } from "@/utils/discordMarkdown";
import AdminPageShell from "../layouts/AdminPageShell.vue";
import AppButton from "../atoms/AppButton.vue";
import AppInput from "@/components/atoms/AppInput.vue";
import AppTextarea from "@/components/atoms/AppTextarea.vue";
import AppToggle from "@/components/atoms/AppToggle.vue";
import ChannelSelect from "@/components/atoms/ChannelSelect.vue";
import ImagePicker from "@/components/molecules/ImagePicker.vue";

const { guildIdFilter } = useGuildSelector();
const { success: toastOk, error: toastErr } = useToast();
const { confirm } = useConfirm();

const list = ref<Embed[]>([]);
const loading = ref(false);
const saving = ref(false);
const currentId = ref<string | null>(null);
const postChannelId = ref("");
const lastMessageId = ref<string | null>(null);

function emptyForm(): EmbedInput {
  return {
    name: "",
    content: "",
    author_name: "",
    author_icon_url: "",
    author_url: "",
    title: "",
    title_url: "",
    description: "",
    color: 0x5865f2,
    image_url: "",
    thumbnail_url: "",
    footer_text: "",
    footer_icon_url: "",
    show_timestamp: false,
    fields: [],
  };
}

const form = ref<EmbedInput>(emptyForm());

// Couleur : l'API stocke un entier, le picker HTML manipule du hex.
const colorHex = computed<string>({
  get: () => `#${(form.value.color ?? 0x5865f2).toString(16).padStart(6, "0")}`,
  set: (v) => {
    form.value.color = parseInt(v.replace("#", ""), 16);
  },
});

async function reload() {
  if (!guildIdFilter.value) return;
  loading.value = true;
  try {
    list.value = await embedsService.list(guildIdFilter.value);
  } catch (e) {
    toastErr(`Chargement : ${errMsg(e)}`);
  } finally {
    loading.value = false;
  }
}
watch(guildIdFilter, reload, { immediate: true });

function newEmbed() {
  currentId.value = null;
  lastMessageId.value = null;
  form.value = emptyForm();
}

function loadEmbed(e: Embed) {
  currentId.value = e.id;
  lastMessageId.value = e.last_message_id;
  form.value = {
    name: e.name,
    content: e.content,
    author_name: e.author_name,
    author_icon_url: e.author_icon_url,
    author_url: e.author_url,
    title: e.title,
    title_url: e.title_url,
    description: e.description,
    color: e.color,
    image_url: e.image_url,
    thumbnail_url: e.thumbnail_url,
    footer_text: e.footer_text,
    footer_icon_url: e.footer_icon_url,
    show_timestamp: e.show_timestamp,
    fields: e.fields.map((f) => ({ ...f })),
  };
}

function addField() {
  if (form.value.fields.length >= 25) return;
  form.value.fields.push({ name: "", value: "", inline: false });
}
function removeField(i: number) {
  form.value.fields.splice(i, 1);
}

async function save() {
  if (!guildIdFilter.value) return;
  if (!form.value.name.trim()) {
    toastErr("Donne un nom à l'embed.");
    return;
  }
  saving.value = true;
  try {
    const saved = currentId.value
      ? await embedsService.update(currentId.value, form.value)
      : await embedsService.create(guildIdFilter.value, form.value);
    currentId.value = saved.id;
    lastMessageId.value = saved.last_message_id;
    toastOk("Embed enregistré.");
    await reload();
  } catch (e) {
    toastErr(`Enregistrement : ${errMsg(e)}`);
  } finally {
    saving.value = false;
  }
}

async function removeEmbed() {
  if (!currentId.value) return;
  if (!(await confirm({ title: "Supprimer", message: `Supprimer l'embed « ${form.value.name} » ?` }))) return;
  try {
    await embedsService.remove(currentId.value);
    toastOk("Embed supprimé.");
    newEmbed();
    await reload();
  } catch (e) {
    toastErr(`Suppression : ${errMsg(e)}`);
  }
}

async function post() {
  if (!currentId.value) {
    toastErr("Enregistre l'embed d'abord.");
    return;
  }
  if (!postChannelId.value) {
    toastErr("Choisis un salon.");
    return;
  }
  try {
    await embedsService.post(currentId.value, postChannelId.value);
    toastOk("Embed envoyé dans le salon 🎉");
    // Le bot rapporte le message posté de façon asynchrone : on recharge après
    // un court délai pour récupérer last_message_id (activer « Mettre à jour »).
    setTimeout(reload, 1500);
  } catch (e) {
    toastErr(`Envoi : ${errMsg(e)}`);
  }
}

async function updatePosted() {
  if (!currentId.value) return;
  try {
    await embedsService.editPosted(currentId.value);
    toastOk("Message mis à jour.");
  } catch (e) {
    toastErr(`Mise à jour : ${errMsg(e)}`);
  }
}
</script>

<template>
  <AdminPageShell title="Embed builder" icon="🎨" width="wide">
    <div class="eb-layout">
      <!-- Liste des embeds sauvegardés -->
      <aside class="eb-list card">
        <div class="eb-list-head">
          <h3>Mes embeds</h3>
          <AppButton size="xs" variant="primary" @click="newEmbed">+ Nouveau</AppButton>
        </div>
        <p v-if="loading" class="muted small">Chargement…</p>
        <p v-else-if="list.length === 0" class="muted small">Aucun embed. Crée le premier !</p>
        <ul v-else class="eb-items">
          <li
            v-for="e in list"
            :key="e.id"
            :class="{ active: e.id === currentId }"
            @click="loadEmbed(e)"
          >
            {{ e.name || "(sans nom)" }}
          </li>
        </ul>
      </aside>

      <!-- Formulaire -->
      <section class="eb-form card">
        <label>Nom (interne)
          <AppInput v-model="form.name" placeholder="ex: Règlement, Annonce event…" />
        </label>
        <label>Message texte (au-dessus de la carte, optionnel)
          <AppTextarea v-model="form.content" :rows="2" />
        </label>

        <fieldset>
          <legend>Author (en-tête)</legend>
          <div class="grid-2">
            <label>Nom<AppInput v-model="form.author_name" /></label>
            <label>URL du nom (lien)<AppInput v-model="form.author_url" placeholder="https://…" /></label>
          </div>
          <label>Icône (URL)<AppInput v-model="form.author_icon_url" placeholder="https://…" /></label>
        </fieldset>

        <fieldset>
          <legend>Corps</legend>
          <div class="grid-2">
            <label>Titre<AppInput v-model="form.title" /></label>
            <label>URL du titre (lien)<AppInput v-model="form.title_url" placeholder="https://…" /></label>
          </div>
          <label>Description<AppTextarea v-model="form.description" :rows="5" /></label>
          <div class="grid-2">
            <label>Couleur<input v-model="colorHex" type="color" /></label>
            <label class="tstamp">
              <span>Horodatage</span>
              <AppToggle v-model="form.show_timestamp" />
            </label>
          </div>
          <label>Image (grande)<ImagePicker v-model="form.image_url" /></label>
          <label>Vignette (petite, à droite)<ImagePicker v-model="form.thumbnail_url" /></label>
        </fieldset>

        <fieldset>
          <legend>Champs ({{ form.fields.length }}/25)</legend>
          <div v-for="(f, i) in form.fields" :key="i" class="eb-field">
            <div class="grid-2">
              <label>Nom<AppInput v-model="f.name" /></label>
              <label class="inline-row">
                <span>Inline</span>
                <AppToggle v-model="f.inline" />
              </label>
            </div>
            <label>Valeur<AppTextarea v-model="f.value" :rows="2" /></label>
            <button type="button" class="link-danger" @click="removeField(i)">Supprimer ce champ</button>
          </div>
          <AppButton size="xs" variant="secondary" :disabled="form.fields.length >= 25" @click="addField">
            + Ajouter un champ
          </AppButton>
        </fieldset>

        <fieldset>
          <legend>Footer (bas)</legend>
          <label>Texte<AppInput v-model="form.footer_text" /></label>
          <label>Icône (URL)<AppInput v-model="form.footer_icon_url" placeholder="https://…" /></label>
        </fieldset>

        <div class="eb-actions">
          <AppButton variant="primary" :disabled="saving" @click="save">
            {{ saving ? "…" : (currentId ? "Enregistrer" : "Créer") }}
          </AppButton>
          <AppButton v-if="currentId" variant="danger" @click="removeEmbed">Supprimer</AppButton>
        </div>

        <div v-if="currentId" class="eb-post card">
          <h4>Poster</h4>
          <div class="grid-2">
            <label>Salon<ChannelSelect v-model="postChannelId" :guild-id="guildIdFilter ?? null" /></label>
            <div class="eb-post-btns">
              <AppButton variant="primary" @click="post">📤 Poster maintenant</AppButton>
              <AppButton v-if="lastMessageId" variant="secondary" @click="updatePosted">
                ♻️ Mettre à jour le message posté
              </AppButton>
            </div>
          </div>
          <p class="muted small">Enregistre tes changements avant de poster / mettre à jour.</p>
        </div>
      </section>

      <!-- Aperçu live -->
      <section class="eb-preview">
        <h3 class="muted small">Aperçu</h3>
        <!-- eslint-disable-next-line vue/no-v-html -- contenu échappé par renderDiscordMarkdown -->
        <p v-if="form.content" class="eb-content" v-html="renderDiscordMarkdown(form.content)"></p>
        <div class="eb-embed" :style="{ borderLeftColor: colorHex }">
          <div class="eb-embed-body">
            <div v-if="form.author_name" class="eb-author">
              <img v-if="form.author_icon_url" :src="form.author_icon_url" class="eb-author-icon" alt="" />
              <span>{{ form.author_name }}</span>
            </div>
            <div v-if="form.title" class="eb-title">{{ form.title }}</div>
            <!-- eslint-disable-next-line vue/no-v-html -- contenu échappé par renderDiscordMarkdown -->
            <div v-if="form.description" class="eb-desc" v-html="renderDiscordMarkdown(form.description)"></div>
            <div v-if="form.fields.length" class="eb-fields">
              <div
                v-for="(f, i) in form.fields.filter((x) => x.name)"
                :key="i"
                class="eb-field-view"
                :class="{ inline: f.inline }"
              >
                <div class="eb-field-name">{{ f.name }}</div>
                <!-- eslint-disable-next-line vue/no-v-html -- contenu échappé par renderDiscordMarkdown -->
                <div class="eb-field-value" v-html="renderDiscordMarkdown(f.value)"></div>
              </div>
            </div>
            <img v-if="form.image_url" :src="form.image_url" class="eb-image" alt="" />
            <div v-if="form.footer_text || form.show_timestamp" class="eb-footer">
              <img v-if="form.footer_icon_url" :src="form.footer_icon_url" class="eb-footer-icon" alt="" />
              <span>{{ form.footer_text }}<template v-if="form.footer_text && form.show_timestamp"> • </template><template v-if="form.show_timestamp">aujourd'hui</template></span>
            </div>
          </div>
          <img v-if="form.thumbnail_url" :src="form.thumbnail_url" class="eb-thumb" alt="" />
        </div>
      </section>
    </div>
  </AdminPageShell>
</template>

<style scoped>
.eb-layout {
  display: grid;
  /* Colonne d'apercu dimensionnee pour l'embed a largeur fixe (440px + marge). */
  grid-template-columns: 200px minmax(0, 1fr) 480px;
  gap: 16px;
  align-items: start;
}
.card {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg, 12px);
  padding: 16px;
}
.eb-list-head { display: flex; justify-content: space-between; align-items: center; margin-bottom: 10px; }
.eb-list-head h3 { margin: 0; font-size: 14px; }
.eb-items { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 4px; }
.eb-items li {
  padding: 8px 10px; border-radius: 8px; cursor: pointer; font-size: 13px;
  background: var(--bg-card); border: 1px solid transparent;
}
.eb-items li:hover { border-color: var(--accent); }
.eb-items li.active { border-color: var(--accent); background: color-mix(in srgb, var(--accent) 15%, transparent); }

.eb-form { display: flex; flex-direction: column; gap: 12px; }
.eb-form label { display: flex; flex-direction: column; gap: 4px; font-size: 12px; font-weight: 600; color: var(--text-secondary); }
.eb-form fieldset { border: 1px solid var(--border); border-radius: 8px; padding: 12px; display: flex; flex-direction: column; gap: 10px; }
.eb-form legend { padding: 0 6px; font-size: 12px; font-weight: 700; }
.grid-2 { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }
.tstamp, .inline-row { flex-direction: row !important; align-items: center; justify-content: space-between; }
.eb-field { border: 1px dashed var(--border); border-radius: 8px; padding: 8px; display: flex; flex-direction: column; gap: 8px; }
.link-danger { align-self: flex-start; background: none; border: none; color: var(--danger); cursor: pointer; font-size: 12px; text-decoration: underline; padding: 0; }
.eb-actions { display: flex; gap: 8px; }
.eb-post { margin-top: 6px; }
.eb-post h4 { margin: 0 0 8px; font-size: 13px; }
.eb-post-btns { display: flex; flex-direction: column; gap: 6px; justify-content: flex-end; }
input[type="color"] { height: 38px; padding: 3px; cursor: pointer; width: 100%; }

/* Aperçu façon Discord */
.eb-preview { position: sticky; top: 16px; }
.eb-content { color: var(--text-primary); font-size: 14px; margin: 4px 0 8px; white-space: pre-wrap; }
.eb-embed {
  display: flex; gap: 12px; justify-content: space-between;
  background: #2b2d31; border-left: 4px solid var(--accent);
  border-radius: 6px; padding: 12px 14px;
  /* Largeur FIXE et identique pour tous les embeds (rendu uniforme dans le
     builder). La hauteur, elle, suit le contenu. */
  width: 440px;
  max-width: 100%;
}
.eb-embed-body { min-width: 0; flex: 1; }
.eb-author { display: flex; align-items: center; gap: 8px; font-size: 13px; font-weight: 600; color: #fff; margin-bottom: 6px; }
.eb-author-icon { width: 20px; height: 20px; border-radius: 50%; }
.eb-title { color: #00a8fc; font-weight: 700; font-size: 15px; margin-bottom: 6px; }
.eb-desc { color: #dbdee1; font-size: 13px; line-height: 1.4; }

/* Rendu markdown Discord dans l'aperçu */
.eb-desc :deep(a), .eb-field-value :deep(a), .eb-content :deep(a) { color: #00a8fc; text-decoration: none; }
.eb-desc :deep(a):hover, .eb-field-value :deep(a):hover { text-decoration: underline; }
.eb-desc :deep(strong), .eb-field-value :deep(strong) { font-weight: 700; color: #fff; }
.eb-desc :deep(.md-code), .eb-field-value :deep(.md-code), .eb-content :deep(.md-code) {
  background: #1e1f22; border-radius: 4px; padding: 1px 4px; font-family: monospace; font-size: 12px;
}
.eb-desc :deep(.md-pre), .eb-field-value :deep(.md-pre) {
  background: #1e1f22; border: 1px solid #111214; border-radius: 4px; padding: 8px;
  font-family: monospace; font-size: 12px; white-space: pre-wrap; margin: 4px 0; overflow-x: auto;
}
.eb-desc :deep(.md-h1) { font-size: 18px; font-weight: 700; color: #fff; margin: 4px 0; }
.eb-desc :deep(.md-h2) { font-size: 16px; font-weight: 700; color: #fff; margin: 4px 0; }
.eb-desc :deep(.md-h3) { font-size: 14px; font-weight: 700; color: #fff; margin: 4px 0; }
.eb-desc :deep(.md-quote), .eb-field-value :deep(.md-quote) {
  border-left: 3px solid #4e5058; padding-left: 8px; margin: 2px 0; color: #b5bac1;
}
.eb-desc :deep(.md-li), .eb-field-value :deep(.md-li) { padding-left: 4px; }
.eb-fields { display: flex; flex-wrap: wrap; gap: 8px; margin-top: 8px; }
.eb-field-view { flex: 1 1 100%; }
.eb-field-view.inline { flex: 1 1 30%; min-width: 120px; }
.eb-field-name { color: #fff; font-weight: 700; font-size: 12px; }
.eb-field-value { color: #dbdee1; font-size: 12px; white-space: pre-wrap; }
.eb-image { max-width: 100%; border-radius: 6px; margin-top: 10px; display: block; }
.eb-thumb { width: 72px; height: 72px; border-radius: 6px; object-fit: cover; flex-shrink: 0; }
.eb-footer { display: flex; align-items: center; gap: 6px; margin-top: 8px; color: #949ba4; font-size: 11px; }
.eb-footer-icon { width: 18px; height: 18px; border-radius: 50%; }
.muted { color: var(--text-secondary); }
.small { font-size: 12px; }

@media (max-width: 1100px) {
  .eb-layout { grid-template-columns: 1fr; }
  .eb-preview { position: static; }
}
</style>
