// Rendu d'un sous-ensemble du markdown Discord, pour l'apercu d'embed.
//
// SECURITE : on echappe TOUJOURS le HTML de l'entree d'abord, puis on
// n'introduit que des balises connues. Les liens sont restreints a http(s).
// -> aucune injection possible depuis le contenu saisi par l'utilisateur.

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

/// Transforme le markdown Discord en HTML sur (a passer a v-html).
export function renderDiscordMarkdown(input: string): string {
  if (!input) return "";
  let s = escapeHtml(input);

  // Blocs de code ```lang\n...``` (proteges des autres regles : on les traite
  // en premier et leur contenu n'est plus reformate).
  s = s.replace(/```(?:[a-zA-Z0-9+-]*\n)?([\s\S]*?)```/g, (_m, code: string) => {
    return `<pre class="md-pre"><code>${code.replace(/\n$/, "")}</code></pre>`;
  });
  // Code inline `...`
  s = s.replace(/`([^`\n]+?)`/g, '<code class="md-code">$1</code>');

  // Titres (Discord : #, ##, ###) en debut de ligne.
  s = s.replace(/^### (.*)$/gm, '<div class="md-h3">$1</div>');
  s = s.replace(/^## (.*)$/gm, '<div class="md-h2">$1</div>');
  s = s.replace(/^# (.*)$/gm, '<div class="md-h1">$1</div>');

  // Citations « > … » (le > a ete echappe en &gt;).
  s = s.replace(/^&gt; (.*)$/gm, '<div class="md-quote">$1</div>');

  // Listes a puces « - » ou « * ».
  s = s.replace(/^[*-] (.*)$/gm, '<div class="md-li">• $1</div>');

  // Gras, souligne, barre, italique (ordre important).
  s = s.replace(/\*\*([^*]+?)\*\*/g, "<strong>$1</strong>");
  s = s.replace(/__([^_]+?)__/g, "<u>$1</u>");
  s = s.replace(/~~([^~]+?)~~/g, "<del>$1</del>");
  s = s.replace(/\*([^*\n]+?)\*/g, "<em>$1</em>");
  s = s.replace(/_([^_\n]+?)_/g, "<em>$1</em>");

  // Liens [texte](url) — url restreinte a http(s).
  s = s.replace(
    /\[([^\]]+)\]\((https?:\/\/[^\s)]+)\)/g,
    '<a href="$2" target="_blank" rel="noopener noreferrer">$1</a>',
  );

  // Sauts de ligne -> <br>, en nettoyant ceux colles aux blocs.
  s = s.replace(/\n/g, "<br>");
  s = s.replace(/(<\/(?:div|pre)>)<br>/g, "$1");
  s = s.replace(/<br>(<(?:div|pre)\b)/g, "$1");

  return s;
}
