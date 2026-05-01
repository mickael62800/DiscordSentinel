import { httpDelete, httpGet, httpPost } from "@/api/http";

export interface InvitationDto {
  code: string;
  guild_id: string;
  role: "viewer" | "moderator" | "admin" | "owner";
  created_by: string;
  created_at: string;
  expires_at: string | null;
  used_at: string | null;
  used_by_discord_id: string | null;
  notes: string | null;
  status: "active" | "used" | "expired";
}

export interface CreateInvitationParams {
  guild_id: string;
  role: "viewer" | "moderator" | "admin" | "owner";
  /** Heures avant expiration (defaut 168 = 7 jours, 0 = pas d'expiration) */
  expires_in_hours?: number;
  notes?: string;
}

export interface RedeemInvitationParams {
  code: string;
}

export interface RedeemInvitationResponse {
  guild_id: string;
  role: string;
  message: string;
}

export interface CheckAccessResponse {
  is_authorized: boolean;
  is_superadmin: boolean;
  guild_count: number;
  message: string;
}

export const invitationsService = {
  create(params: CreateInvitationParams): Promise<InvitationDto> {
    return httpPost("/api/invitations", params);
  },
  list(guildId: string): Promise<InvitationDto[]> {
    return httpGet(`/api/invitations/${guildId}`);
  },
  revoke(code: string): Promise<unknown> {
    return httpDelete(`/api/invitations/code/${code}`);
  },
  redeem(code: string): Promise<RedeemInvitationResponse> {
    return httpPost("/api/auth/redeem-invitation", { code });
  },
  checkAccess(): Promise<CheckAccessResponse> {
    return httpGet("/api/auth/check-access");
  },
};
