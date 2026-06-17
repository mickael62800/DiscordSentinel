import { httpDelete, httpGet } from "@/api/http";

/** Compagnon Tamagotchi (miroir de PetDto cote API). */
export interface Pet {
  id: string;
  guild_id: string;
  owner_id: string;
  name: string;
  species: string;
  level: number;
  xp: number;
  xp_in_level: number;
  xp_for_level: number;
  born_at: string;
  hunger: number;
  happiness: number;
  energy: number;
  status: "healthy" | "sick" | "dead" | string;
  str: number;
  vit: number;
  agi: number;
  elo: number;
  wins: number;
  losses: number;
  cooldowns: Record<string, unknown>;
}

export const tamagotchiService = {
  /** GET /api/tamagotchi/{guild_id}/pets — tous les compagnons de la guild (admin+). */
  list(guildId: string): Promise<Pet[]> {
    return httpGet(`/api/tamagotchi/${guildId}/pets`);
  },
  /** DELETE /api/tamagotchi/{guild_id}/pets/{pet_id} — supprime un compagnon (owner+). */
  delete(guildId: string, petId: string): Promise<void> {
    return httpDelete(`/api/tamagotchi/${guildId}/pets/${petId}`);
  },
};
