import { httpGet, httpPost } from "@/api/http";
import type { Wallet } from "@/types";

export const walletService = {
  list(guildId: string): Promise<Wallet[]> {
    return httpGet(`/api/wallets/${guildId}`);
  },
  credit(guildId: string, userId: string, amount: number, description?: string): Promise<Wallet> {
    return httpPost(`/api/wallet/${guildId}/${userId}/credit`, { amount, source: "admin", description });
  },
  debit(guildId: string, userId: string, amount: number, description?: string): Promise<Wallet> {
    return httpPost(`/api/wallet/${guildId}/${userId}/debit`, { amount, source: "admin", description });
  },
  reset(guildId: string, userId: string, newBalance: number): Promise<Wallet> {
    return httpPost(`/api/wallets/${guildId}/${userId}/reset`, { new_balance: newBalance });
  },
  async resetAll(guildId: string, newBalance: number): Promise<number> {
    const v = await httpPost<{ affected?: number }>(`/api/wallets/${guildId}/reset-all`, { new_balance: newBalance });
    return v?.affected ?? 0;
  },
};
