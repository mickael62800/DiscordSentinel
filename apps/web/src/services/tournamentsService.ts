import { httpGet } from "@/api/http";

export interface TournamentStanding {
  user_id: string;
  username: string;
  net_gain: number;
  rank: number;
}

export interface CurrentTournament {
  guild_id: string;
  week_start: string;
  week_end: string;
  prize_pool_estimated: number;
  standings: TournamentStanding[];
}

export interface PastTournament {
  id: string;
  guild_id: string;
  week_start: string;
  week_end: string;
  winner_user_id: string | null;
  winner_username: string | null;
  winner_net_gain: number;
  prize_amount: number;
  status: string;
  resolved_at: string | null;
}

export const tournamentsService = {
  current(guildId: string): Promise<CurrentTournament> {
    return httpGet(`/api/coude/${guildId}/tournaments/current`);
  },
  history(guildId: string): Promise<PastTournament[]> {
    return httpGet(`/api/coude/${guildId}/tournaments/history`);
  },
};
