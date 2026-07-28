import { httpGet, httpPost } from "@/api/http";
import type { Member, MemberSummary } from "@/types";

export interface ResetMemberResult {
  status: string;
  guild_id: string;
  user_id: string;
  totals: Record<string, number>;
}

export const membersService = {
  getAll(guildId: string): Promise<Member[]> {
    return httpGet(`/api/members/${guildId}`);
  },
  getSummary(guildId: string, userId: string): Promise<MemberSummary> {
    return httpGet(`/api/members/${guildId}/${userId}/summary`);
  },
  /**
   * Reinitialise completement un membre : supprime infractions, actions de
   * moderation, strikes, notes, surveillance et rappels.
   * Irreversible, necessite admin+.
   */
  resetMember(guildId: string, userId: string): Promise<ResetMemberResult> {
    return httpPost(`/api/members/${guildId}/${userId}/reset`);
  },
};
