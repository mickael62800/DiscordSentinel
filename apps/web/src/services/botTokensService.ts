import { saveBotToken, deleteBotToken, listBotTokens } from "@/api/config";

export const botTokensService = {
  save(botName: string, token: string) { saveBotToken(botName, token); },
  remove(botName: string) { deleteBotToken(botName); },
  list(): Array<[string, boolean]> { return listBotTokens(); },
};
