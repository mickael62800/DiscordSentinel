import { httpGet, httpPost, httpPatch, httpDelete } from "@/api/http";
import type { Ticket, TicketDetail } from "@/types";
import { q } from "./_query";

export interface BulkDeleteParams {
  author_id?: string | null;
  from?: string | null;
  to?: string | null;
  all?: boolean;
}

export interface BulkDeleteResult {
  deleted: number;
  author_id: string | null;
  from: string | null;
  to: string | null;
}

export const ticketsService = {
  getAll(): Promise<Ticket[]> { return httpGet("/api/tickets"); },
  getDetail(id: string): Promise<TicketDetail> { return httpGet(`/api/tickets/${id}`); },
  reply(ticketId: string, content: string): Promise<unknown> {
    return httpPost(`/api/tickets/${ticketId}/messages`, { content });
  },
  close(id: string): Promise<unknown> { return httpPatch(`/api/tickets/${id}/close`); },
  assign(id: string, assignee: string): Promise<unknown> {
    return httpPatch(`/api/tickets/${id}/assign`, { assignee });
  },
  /**
   * Supprime en masse selon des filtres optionnels (author_id, from, to).
   * Si aucun filtre fourni, il faut passer `all=true` pour autoriser
   * une suppression totale — sinon le backend renvoie une erreur de
   * validation (protection contre un DELETE accidentel).
   */
  bulkDelete(params: BulkDeleteParams): Promise<BulkDeleteResult> {
    return httpDelete(
      `/api/tickets/bulk${q({
        author_id: params.author_id ?? null,
        from: params.from ?? null,
        to: params.to ?? null,
        all: params.all ? "true" : null,
      })}`,
    );
  },
};
