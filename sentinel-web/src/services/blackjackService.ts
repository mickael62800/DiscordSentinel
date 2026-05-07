import { httpGet, httpDelete } from "@/api/http";
import type { BlackjackGame } from "@/types";
import { q } from "./_query";

export interface BlackjackTable {
  id: string;
  guild_id: string;
  channel_id: string;
  owner_id: string;
  owner_name: string;
  status: string;
  created_at: string;
}

export interface BlackjackTablePlayer {
  user_id: string;
  user_name: string;
  joined_at: string;
}

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
  listTables(guildId: string): Promise<BlackjackTable[]> {
    return httpGet(`/api/blackjack/admin/${guildId}/tables`);
  },
  listTablePlayers(tableId: string): Promise<BlackjackTablePlayer[]> {
    return httpGet(`/api/blackjack/tables/${tableId}/players`);
  },
  closeTable(tableId: string): Promise<void> {
    return httpDelete(`/api/blackjack/tables/${tableId}`);
  },
};
