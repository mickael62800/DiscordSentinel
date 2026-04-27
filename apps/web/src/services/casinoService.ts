import { httpGet } from "@/api/http";
import { q } from "./_query";
import type {
  JackpotPool,
  SlotSpin,
  SlotTopWinner,
  WheelSpinLog,
  WheelTopWinner,
} from "@/types/casino";

export const slotService = {
  recentSpins(guildId: string, limit = 30): Promise<SlotSpin[]> {
    return httpGet(`/api/slot/${guildId}/recent${q({ limit })}`);
  },
  topWinners(guildId: string, days = 7, limit = 10): Promise<SlotTopWinner[]> {
    return httpGet(
      `/api/slot/${guildId}/leaderboard${q({ days, limit })}`,
    );
  },
  jackpot(guildId: string): Promise<JackpotPool> {
    return httpGet(`/api/slot/${guildId}/jackpot`);
  },
};

export const wheelService = {
  recentSpins(guildId: string, limit = 30): Promise<WheelSpinLog[]> {
    return httpGet(`/api/wheel/${guildId}/recent${q({ limit })}`);
  },
  topWinners(guildId: string, days = 7, limit = 10): Promise<WheelTopWinner[]> {
    return httpGet(
      `/api/wheel/${guildId}/leaderboard${q({ days, limit })}`,
    );
  },
};
