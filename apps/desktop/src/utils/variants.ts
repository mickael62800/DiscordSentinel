export type BadgeVariant = "danger" | "warning" | "info" | "success" | "default";

export function severityVariant(severity: string): BadgeVariant {
  switch (severity) {
    case "critical":
    case "urgent":
      return "danger";
    case "high":
      return "warning";
    case "medium":
      return "info";
    case "low":
      return "default";
    default:
      return "default";
  }
}

export function actionVariant(action: string): BadgeVariant {
  switch (action) {
    case "ban":
    case "ban_permanent":
    case "ban_temp":
    case "lockdown":
      return "danger";
    case "mute":
    case "mute_permanent":
    case "mute_temp":
    case "delete":
      return "warning";
    case "warn":
      return "info";
    case "unban":
    case "unmute":
      return "success";
    default:
      return "default";
  }
}

export function statusVariant(status: string): BadgeVariant {
  switch (status) {
    case "open":
      return "info";
    case "pending":
      return "warning";
    case "closed":
      return "success";
    default:
      return "default";
  }
}

export function priorityVariant(priority: string): BadgeVariant {
  switch (priority) {
    case "urgent":
      return "danger";
    case "high":
      return "warning";
    case "medium":
      return "info";
    case "low":
      return "default";
    default:
      return "default";
  }
}

export function levelVariant(level: string): BadgeVariant {
  if (level === "info" || level === "warn" || level === "error") {
    return level === "error" ? "danger" : level as BadgeVariant;
  }
  return "default";
}

export function infractionTypeVariant(type: string): BadgeVariant {
  switch (type) {
    case "ban":
      return "danger";
    case "mute":
      return "warning";
    case "warn":
      return "info";
    default:
      return "default";
  }
}
