import { httpGet } from "@/api/http";
import type { Member, MemberSummary } from "@/types";

export const membersService = {
  getAll(guildId: string): Promise<Member[]> {
    return httpGet(`/api/members/${guildId}`);
  },
  getSummary(guildId: string, userId: string): Promise<MemberSummary> {
    return httpGet(`/api/members/${guildId}/${userId}/summary`);
  },
};
