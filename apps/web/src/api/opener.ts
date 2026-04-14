// Ouverture de lien en nouvel onglet — ouvre un lien dans un nouvel onglet.
export async function openUrl(url: string): Promise<void> {
  window.open(url, "_blank", "noopener,noreferrer");
}
export async function openPath(path: string): Promise<void> {
  window.open(path, "_blank", "noopener,noreferrer");
}
