import { httpGet, httpPost, httpPatch } from "@/api/http";
import type { ModerationRule, UpdateRuleParams } from "@/types";
import { q } from "./_query";

export const rulesService = {
  getAll(guildId?: string | null): Promise<ModerationRule[]> {
    return httpGet(`/api/rules${q({ guild_id: guildId ?? null })}`);
  },

  async toggle(id: string, enabled: boolean): Promise<boolean> {
    await httpPatch(`/api/rules/${id}`, { enabled });
    return enabled;
  },

  update(params: UpdateRuleParams): Promise<unknown> {
    if (params.weight < 0 || params.weight > 10) throw new Error("Le poids doit etre entre 0 et 10");
    for (const [n, v] of [
      ["warn", params.threshold_warn],
      ["delete", params.threshold_delete],
      ["mute", params.threshold_mute],
      ["ban", params.threshold_ban],
    ] as const) {
      if (v < 0 || v > 100) throw new Error(`Le seuil ${n} doit etre entre 0 et 100`);
    }
    return httpPost("/rules", {
      guild_id: params.guild_id,
      flag_type: params.flag_type,
      weight: params.weight,
      threshold_warn: params.threshold_warn,
      threshold_delete: params.threshold_delete,
      threshold_mute: params.threshold_mute,
      threshold_ban: params.threshold_ban,
      enabled: params.enabled,
    });
  },
};
