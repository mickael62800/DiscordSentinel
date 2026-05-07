import { httpDelete, httpGet, httpPost } from "@/api/http";
import type { AddNotePayload, UserNote } from "@/types/notes";

export const notesService = {
  list(guildId: string, userId: string): Promise<UserNote[]> {
    return httpGet(`/api/notes/${guildId}/${userId}`);
  },
  add(body: AddNotePayload): Promise<UserNote> {
    return httpPost("/api/notes", body);
  },
  remove(id: string): Promise<unknown> {
    return httpDelete(`/api/notes/${id}`);
  },
};
