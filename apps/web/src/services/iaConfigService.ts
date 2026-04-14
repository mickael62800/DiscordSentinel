import { httpGet, httpPut } from "@/api/http";
import type { IaConfig, SaveIaConfigParams } from "@/types";

export const iaConfigService = {
  get(guildId: string): Promise<IaConfig> {
    return httpGet(`/api/ia-config/${guildId}`);
  },
  save(guildId: string, params: SaveIaConfigParams): Promise<IaConfig> {
    return httpPut(`/api/ia-config/${guildId}`, {
      text_enabled: params.text_enabled,
      text_threshold: params.text_threshold,
      vision_enabled: params.vision_enabled,
      vision_threshold: params.vision_threshold,
      context_dampening: params.context_dampening,
      context_format: params.context_format,
      context_max_messages: params.context_max_messages,
      context_max_chars: params.context_max_chars,
    });
  },
};
