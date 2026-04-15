import { httpGet, httpDelete } from "@/api/http";
import type { BlackjackGame } from "@/types";
import { q } from "./_query";

export const blackjackService = {
  listGames(guildId: string, status?: string | null): Promise<BlackjackGame[]> {
    return httpGet(`/api/blackjack/admin/${guildId}/games${q({ status })}`);
  },
  cancelGame(gameId: string): Promise<void> {
    return httpDelete(`/api/blackjack/admin/games/${gameId}`);
  },
  purgeAll(guildId: string): Promise<{ deleted_games: number; deleted_tables: number }> {
    return httpDelete(`/api/blackjack/admin/${guildId}/purge`);
  },
};
