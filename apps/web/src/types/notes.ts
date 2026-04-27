export interface UserNote {
  id: string;
  guild_id: string;
  user_id: string;
  author_id: string;
  author_name: string;
  content: string;
  category: string; // "general" | "warning" | "positive" | "context"
  created_at: string;
  updated_at: string;
}

export interface AddNotePayload {
  guild_id: string;
  user_id: string;
  author_id: string;
  author_name: string;
  content: string;
  category?: string;
}
