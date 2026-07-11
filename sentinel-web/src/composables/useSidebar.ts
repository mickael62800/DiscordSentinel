import { ref } from "vue";

// Etat d'ouverture du drawer de navigation (mobile). Sur desktop la sidebar est
// toujours visible via CSS ; ce flag ne pilote que l'overlay mobile.
// Module-scoped = singleton partage entre le bouton hamburger (TopBar) et la
// Sidebar elle-meme.
const open = ref(false);

// Groupes repliés dans la sidebar, persistés en localStorage pour survivre au
// rechargement. Contient les préfixes de groupe (ex: "community").
const K_COLLAPSED = "ds.sidebar.collapsed";
function loadCollapsed(): Set<string> {
  try {
    const raw = localStorage.getItem(K_COLLAPSED);
    return new Set(raw ? (JSON.parse(raw) as string[]) : []);
  } catch {
    return new Set();
  }
}
const collapsed = ref<Set<string>>(loadCollapsed());

export function useSidebar() {
  function toggle() {
    open.value = !open.value;
  }
  function close() {
    open.value = false;
  }
  function toggleGroup(prefix: string) {
    const next = new Set(collapsed.value);
    if (next.has(prefix)) next.delete(prefix);
    else next.add(prefix);
    collapsed.value = next;
    try {
      localStorage.setItem(K_COLLAPSED, JSON.stringify([...next]));
    } catch {
      /* quota / mode privé : on ignore, l'état reste en mémoire */
    }
  }
  function isCollapsed(prefix: string): boolean {
    return collapsed.value.has(prefix);
  }
  return { open, toggle, close, collapsed, toggleGroup, isCollapsed };
}
