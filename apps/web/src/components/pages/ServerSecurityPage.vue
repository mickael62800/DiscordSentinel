<script setup lang="ts">
/**
 * Page Securite serveur — surveillance des attaques et de l'integrite.
 *
 * Sections (a implementer une a une) :
 *  - Top IPs par requetes (1h / 24h)
 *  - Echecs d'auth recents (401/403)
 *  - IPs bannies (fail2ban / ufw)
 *  - Audit log admin (qui a fait quoi)
 *  - Certificat TLS (expiration)
 *
 * Acces : admin+ (gated via RBAC registry config.security-monitoring).
 */
import { ref } from "vue";

const refreshing = ref(false);

async function refreshAll() {
  refreshing.value = true;
  // TODO : refetch toutes les sections
  setTimeout(() => { refreshing.value = false; }, 300);
}
</script>

<template>
  <div class="security-page">
    <div class="page-header">
      <div>
        <h1>🛡️ Sécurité serveur</h1>
        <p class="muted">
          Surveillance des attaques, échecs d'authentification, IPs bannies et
          audit des actions administratives.
        </p>
      </div>
      <button class="btn" :disabled="refreshing" @click="refreshAll">
        {{ refreshing ? "Actualisation…" : "↻ Actualiser tout" }}
      </button>
    </div>

    <!-- ── Top IPs ── -->
    <section class="card">
      <h2>📊 Top IPs par requêtes</h2>
      <p class="muted small">À implémenter — top 20 IPs par volume de requêtes (1h / 24h).</p>
      <div class="placeholder">Section en construction</div>
    </section>

    <!-- ── Échecs d'auth ── -->
    <section class="card">
      <h2>🔒 Échecs d'authentification</h2>
      <p class="muted small">À implémenter — liste des 401/403 récents avec IP, endpoint, user-agent.</p>
      <div class="placeholder">Section en construction</div>
    </section>

    <!-- ── IPs bannies ── -->
    <section class="card">
      <h2>🚫 IPs bannies</h2>
      <p class="muted small">À implémenter — liste fail2ban / ufw avec raison + expiration.</p>
      <div class="placeholder">Section en construction</div>
    </section>

    <!-- ── Audit log admin ── -->
    <section class="card">
      <h2>📋 Audit log administratif</h2>
      <p class="muted small">À implémenter — qui a fait quoi (Docker, RBAC, ban, reset, prune).</p>
      <div class="placeholder">Section en construction</div>
    </section>

    <!-- ── TLS / certificats ── -->
    <section class="card">
      <h2>🔐 Certificat TLS</h2>
      <p class="muted small">À implémenter — date d'expiration Let's Encrypt + alerte si proche.</p>
      <div class="placeholder">Section en construction</div>
    </section>
  </div>
</template>

<style scoped>
.security-page {
  padding: 16px;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  flex-wrap: wrap;
  gap: 16px;
  margin-bottom: 20px;
}
.page-header h1 {
  margin: 0 0 4px;
  font-size: 1.6rem;
}
.muted {
  color: var(--text-secondary);
  margin: 0;
}
.muted.small {
  font-size: 12px;
}

.card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 18px 20px;
  margin-bottom: 16px;
}
.card h2 {
  margin: 0 0 8px;
  font-size: 16px;
}

.placeholder {
  margin-top: 12px;
  padding: 20px;
  background: var(--bg-secondary);
  border: 1px dashed var(--border);
  border-radius: 8px;
  text-align: center;
  color: var(--text-secondary);
  font-size: 12px;
  font-style: italic;
}

.btn {
  padding: 8px 14px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg-secondary);
  color: var(--text-primary);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
}
.btn:hover:not(:disabled) {
  border-color: var(--accent);
  color: var(--accent);
}
.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
