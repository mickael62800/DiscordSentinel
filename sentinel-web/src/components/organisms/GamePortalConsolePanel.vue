<script setup lang="ts">
import { ref, watch, nextTick } from "vue";
import type { GameServer } from "@/services/gamePortalService";
import GameServerStatsBar from "@/components/molecules/GameServerStatsBar.vue";

export interface LogLine {
  time: string;
  source: string;
  level: "info" | "warn" | "error" | "sys";
  text: string;
}

const props = defineProps<{
  selectedServer: GameServer | null;
  logs: LogLine[];
}>();

const cmd = defineModel<string>("cmd", { default: "" });

const emit = defineEmits<{
  send: [];
}>();

const consoleEl = ref<HTMLElement | null>(null);

watch(
  () => props.logs.length,
  () => {
    nextTick(() => {
      if (consoleEl.value) consoleEl.value.scrollTop = consoleEl.value.scrollHeight;
    });
  },
);

function onSubmit() {
  if (!cmd.value.trim()) return;
  emit("send");
}
</script>

<template>
  <section class="panel console">
    <div class="panel-head">
      <h2>
        Console
        <span v-if="selectedServer" class="hint">— {{ selectedServer.name }}</span>
      </h2>
      <div class="legend">
        <span class="dot info" /> info
        <span class="dot warn" /> warn
        <span class="dot error" /> error
      </div>
    </div>

    <GameServerStatsBar
      v-if="selectedServer"
      :server-id="selectedServer.id"
      :active="selectedServer.status === 'running'"
      :interval-ms="5000"
    />

    <div ref="consoleEl" class="console-out">
      <div v-for="(l, i) in logs" :key="i" class="line" :class="l.level">
        <span class="t">{{ l.time }}</span>
        <span class="s">[{{ l.source }}]</span>
        <span class="m">{{ l.text }}</span>
      </div>
    </div>

    <form class="console-in" @submit.prevent="onSubmit">
      <span class="prompt">$</span>
      <input
        v-model="cmd"
        type="text"
        placeholder="Entrez une commande pour le serveur sélectionné…"
        :disabled="!selectedServer || selectedServer.status !== 'running'"
      />
      <button type="submit" :disabled="!cmd.trim()">Envoyer</button>
    </form>
  </section>
</template>

<style scoped>
.panel {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: var(--space-lg);
  display: flex; flex-direction: column;
  min-height: 0;
}
.panel-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--space-md);
  margin-bottom: var(--space-md);
}
.panel-head h2 { margin: 0; font-size: 14px; font-weight: 600; }
.hint { color: var(--text-secondary); font-size: 12px; font-weight: 400; }

.legend {
  display: flex;
  gap: var(--space-md);
  font-size: 11px;
  color: var(--text-secondary);
  align-items: center;
}
.dot { display: inline-block; width: 8px; height: 8px; border-radius: 50%; margin-right: 4px; }
.dot.info { background: var(--info); }
.dot.warn { background: var(--warning); }
.dot.error { background: var(--danger); }

.console-out {
  flex: 1;
  background: var(--bg-primary);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  padding: var(--space-md);
  font-family: 'JetBrains Mono', 'Cascadia Code', 'Consolas', monospace;
  font-size: 12px;
  overflow: auto;
  min-height: 0;
}
.line { display: flex; gap: var(--space-sm); padding: 1px 0; }
.line .t { color: var(--text-secondary); opacity: 0.6; }
.line .s { color: var(--accent-alt); min-width: 70px; }
.line .m { color: var(--text-primary); flex: 1; word-break: break-word; }
.line.warn .m { color: var(--warning); }
.line.error .m { color: var(--danger); }
.line.sys .m { color: var(--success); }

.console-in {
  display: flex;
  gap: var(--space-sm);
  align-items: center;
  margin-top: var(--space-md);
  background: var(--bg-primary);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  padding: 4px var(--space-md);
  transition: border-color var(--transition-fast);
}
.console-in:focus-within { border-color: var(--accent); }
.prompt { color: var(--success); font-family: 'JetBrains Mono', monospace; font-weight: 700; }
.console-in input {
  flex: 1; background: transparent; border: none; outline: none;
  color: var(--text-primary);
  font-family: 'JetBrains Mono', 'Cascadia Code', monospace;
  font-size: 13px;
  padding: 8px 0;
}
.console-in button {
  background: var(--accent); color: #fff; border: none;
  border-radius: var(--radius-sm);
  padding: 6px 14px;
  font-weight: 600; cursor: pointer; font-size: 12px;
  transition: var(--transition-fast);
}
.console-in button:hover:not(:disabled) { background: var(--accent-hover); }
.console-in button:disabled { opacity: 0.4; cursor: not-allowed; }
</style>
