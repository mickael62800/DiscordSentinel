import { httpGet, httpPost, httpPatch } from "@/api/http";
import type { Ticket, TicketDetail } from "@/types";

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
};
