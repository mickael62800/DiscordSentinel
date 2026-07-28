import { httpGet } from "@/api/http";

export interface RotationState {
  guild_id: string;
  state: string;
  current_admin_id: string | null;
  current_admin_since: string | null;
  period_start: string | null;
  next_rotation_at: string | null;
  candidate_id: string | null;
  candidate_offered_at: string | null;
  asked_this_round: string[];
}

export interface ServedEntry {
  user_id: string;
  served_at: string;
}

export const rotationService = {
  getState(guildId: string): Promise<RotationState> {
    return httpGet(`/api/rotation/${guildId}`);
  },
  getHistory(guildId: string): Promise<ServedEntry[]> {
    return httpGet(`/api/rotation/${guildId}/history`);
  },
};
