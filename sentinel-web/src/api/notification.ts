// Notifications via l'API Notification native — utilise l'API Notification du navigateur.

export async function isPermissionGranted(): Promise<boolean> {
  if (typeof Notification === "undefined") return false;
  return Notification.permission === "granted";
}

export async function requestPermission(): Promise<"granted" | "denied" | "default"> {
  if (typeof Notification === "undefined") return "denied";
  const p = await Notification.requestPermission();
  return p;
}

export interface NotificationOptions { title: string; body?: string; icon?: string }
export async function sendNotification(opts: NotificationOptions | string): Promise<void> {
  if (typeof Notification === "undefined") return;
  if (Notification.permission !== "granted") return;
  const o = typeof opts === "string" ? { title: opts } : opts;
  new Notification(o.title, { body: o.body, icon: o.icon });
}
