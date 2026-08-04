import { httpDelete, httpGet, httpPost, httpPut } from "@/api/http";

export interface EmbedField {
  name: string;
  value: string;
  inline: boolean;
}

export interface EmbedInput {
  name: string;
  content: string;
  author_name: string;
  author_icon_url: string;
  author_url: string;
  title: string;
  title_url: string;
  description: string;
  color: number | null;
  image_url: string;
  thumbnail_url: string;
  footer_text: string;
  footer_icon_url: string;
  show_timestamp: boolean;
  fields: EmbedField[];
}

export interface Embed extends EmbedInput {
  id: string;
  guild_id: string;
  last_channel_id: string | null;
  last_message_id: string | null;
  created_at: string;
  updated_at: string;
}

export const embedsService = {
  list(guildId: string): Promise<Embed[]> {
    return httpGet(`/api/embeds/${guildId}`);
  },
  get(id: string): Promise<Embed> {
    return httpGet(`/api/embeds/by-id/${id}`);
  },
  create(guildId: string, body: EmbedInput): Promise<Embed> {
    return httpPost(`/api/embeds/${guildId}`, body);
  },
  update(id: string, body: EmbedInput): Promise<Embed> {
    return httpPut(`/api/embeds/by-id/${id}`, body);
  },
  remove(id: string): Promise<unknown> {
    return httpDelete(`/api/embeds/by-id/${id}`);
  },
  post(id: string, channelId: string): Promise<unknown> {
    return httpPost(`/api/embeds/by-id/${id}/post`, { channel_id: channelId });
  },
  editPosted(id: string): Promise<unknown> {
    return httpPost(`/api/embeds/by-id/${id}/edit`, {});
  },
};
