// Les sections vivantes de l'espace membre : recherche de joueurs, sondages,
// membre du mois, anniversaires, nouveaux venus, annonces.
//
// Toutes les lectures passent par la surface publique : la page s'affiche
// pour un visiteur non connecté. Les ÉCRITURES (publier une annonce, voter,
// dire « je viens ») passent par `api/http.ts`, qui porte la session.

import { httpDelete, httpGet, httpPost } from "@/api/http";
import { publicGet, query } from "./publicHttp";

// ── Recherche de joueurs ──

export interface PublicLfgPost {
  id: string;
  author_name: string;
  game: string;
  slots: number;
  when_text: string;
  description: string | null;
  created_at: string;
  /// Pseudos seuls : l'API ne publie pas les identifiants Discord.
  interested_names: string[];
  remaining_slots: number;
  is_full: boolean;
}

/// Vue authentifiée : porte les identifiants, nécessaires pour savoir si le
/// lecteur s'est déjà manifesté et s'il est l'auteur.
export interface LfgPost extends Omit<PublicLfgPost, "interested_names"> {
  author_id: string;
  game_server_id: string | null;
  is_open: boolean;
  expires_at: string;
  interested: Array<{ user_id: string; username: string }>;
}

export interface CreateLfgInput {
  game: string;
  slots: number;
  when_text?: string;
  description?: string | null;
  game_server_id?: string | null;
  expires_at?: string;
}

// ── Sondages ──

export interface PollOption {
  id: string;
  label: string;
  /// Toujours renseignée : l'API applique sa palette de repli.
  color: string;
  votes: number;
  share: number;
}

export interface Poll {
  id: string;
  question: string;
  description: string | null;
  closes_at: string;
  is_closed: boolean;
  is_open: boolean;
  total_votes: number;
  options: PollOption[];
  my_vote: string | null;
}

// ── Membre du mois ──

export interface Spotlight {
  username: string;
  avatar: string | null;
  /// `AAAA-MM`.
  period: string;
  reason: string;
}

// ── Anniversaires et nouveaux venus ──

export interface Anniversary {
  username: string;
  avatar: string | null;
  years: number;
  joined_at: string;
}

export interface Newcomer {
  username: string;
  avatar: string | null;
  joined_at: string | null;
}

export interface Pulse {
  anniversaries: Anniversary[];
  newcomers: Newcomer[];
}

// ── Annonces ──

export interface NewsItem {
  id: string;
  title: string;
  body: string;
  excerpt: string;
  image_url: string | null;
  is_pinned: boolean;
  published_at: string;
}

// ── Lectures publiques ──

export const communityLifeService = {
  lfg(guildId: string, limit = 6): Promise<PublicLfgPost[]> {
    return publicGet<PublicLfgPost[]>(
      `/lfg/${encodeURIComponent(guildId)}${query({ limit })}`,
    );
  },

  polls(guildId: string, limit = 3): Promise<Poll[]> {
    return publicGet<Poll[]>(`/polls/${encodeURIComponent(guildId)}${query({ limit })}`);
  },

  /// `null` tant que le staff n'a désigné personne : la section se masque.
  spotlight(guildId: string): Promise<Spotlight | null> {
    return publicGet<Spotlight | null>(`/spotlight/${encodeURIComponent(guildId)}`);
  },

  pulse(guildId: string): Promise<Pulse> {
    return publicGet<Pulse>(`/pulse/${encodeURIComponent(guildId)}`);
  },

  news(guildId: string, limit = 3): Promise<NewsItem[]> {
    return publicGet<NewsItem[]>(`/news/${encodeURIComponent(guildId)}${query({ limit })}`);
  },
};

// ── Écritures (session requise) ──

export const communityActionsService = {
  /// Publier une annonce de recherche. Le pseudo de l'auteur est résolu
  /// côté serveur : on ne l'envoie pas, il serait falsifiable.
  createLfg(guildId: string, input: CreateLfgInput): Promise<LfgPost> {
    return httpPost<LfgPost>(`/api/lfg/${encodeURIComponent(guildId)}`, input);
  },

  closeLfg(id: string): Promise<{ ok: boolean }> {
    return httpPost<{ ok: boolean }>(`/api/lfg/detail/${encodeURIComponent(id)}/close`, {});
  },

  deleteLfg(id: string): Promise<{ deleted: boolean }> {
    return httpDelete<{ deleted: boolean }>(`/api/lfg/detail/${encodeURIComponent(id)}`);
  },

  /// « Je viens ». Renvoie l'annonce relue : la liste des intéressés est à
  /// jour même si quelqu'un d'autre a répondu entre-temps.
  joinLfg(id: string): Promise<LfgPost> {
    return httpPost<LfgPost>(`/api/lfg/detail/${encodeURIComponent(id)}/join`, {});
  },

  leaveLfg(id: string): Promise<LfgPost> {
    return httpDelete<LfgPost>(`/api/lfg/detail/${encodeURIComponent(id)}/join`);
  },

  /// Voter. Renvoie le sondage relu, barres à jour.
  vote(pollId: string, optionId: string): Promise<Poll> {
    return httpPost<Poll>(`/api/polls/detail/${encodeURIComponent(pollId)}/vote`, {
      option_id: optionId,
    });
  },

  /// Sondages vus par un membre connecté : `my_vote` est renseigné, ce que
  /// la surface publique ne peut pas faire.
  myPolls(guildId: string): Promise<Poll[]> {
    return httpGet<Poll[]>(`/api/polls/${encodeURIComponent(guildId)}`);
  },
};
