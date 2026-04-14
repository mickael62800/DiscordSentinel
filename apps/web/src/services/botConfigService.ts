import { httpGet, httpPost, httpDelete } from "@/api/http";
import type { BotDefinition, BotGuildConfig } from "@/types";

export const botConfigService = {
  getDefinitions(): Promise<BotDefinition[]> { return httpGet("/api/bots/definitions"); },
  getGuildConfig(guildId: string): Promise<BotGuildConfig[]> {
    return httpGet(`/api/bots/config/${guildId}`);
  },
  set(guildId: string, botName: string, configKey: string, configValue: string): Promise<unknown> {
    return httpPost("/api/bots/config", {
      guild_id: guildId, bot_name: botName, config_key: configKey, config_value: configValue,
    });
  },
  remove(guildId: string, botName: string, configKey: string): Promise<unknown> {
    return httpDelete("/api/bots/config", {
      guild_id: guildId, bot_name: botName, config_key: configKey,
    });
  },
};
