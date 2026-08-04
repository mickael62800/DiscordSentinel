/**
 * Configuration Welcome / Onboarding (mappee 1:1 sur l API
 * sentinel-api/src/adapters/inbound/http/handlers/welcome.rs).
 *
 * 5 sections :
 * - welcome   : message de bienvenue dans un salon (embed + DM)
 * - leave     : message de depart
 * - rules     : verification gate (lecture des regles + role attribue)
 * - counter   : compteur de membres dans un nom de salon
 * - anniversary: anniversaire d arrivee sur le serveur
 */
export interface WelcomeConfig {
  guild_id: string;

  welcome_enabled: boolean;
  welcome_channel_id: string | null;
  welcome_message: string;
  welcome_embed_color: string;
  welcome_dm_enabled: boolean;
  welcome_dm_message: string;
  welcome_title: string;
  welcome_image_url: string;
  welcome_text_position: "above" | "below";
  welcome_footer_text: string;

  leave_enabled: boolean;
  leave_channel_id: string | null;
  leave_message: string;
  leave_title: string;
  leave_image_url: string;
  leave_footer_text: string;
  leave_embed_color: string;

  rules_enabled: boolean;
  rules_channel_id: string | null;
  rules_message: string;
  rules_role_id: string | null;
  rules_button_label: string;
  rules_embed_color: string;

  // Verification d'age au reglement.
  age_check_enabled: boolean;
  age_minimum: number;
  unverified_role_id: string | null;
  age_modal_question: string;
  age_ban_message: string;
  // Bornes de saisie + parametrage du ban (lues directement par le bot).
  age_min: number;
  age_max: number;
  age_ban_days_per_year: number;
  age_ban_log_channel_id: string | null;

  counter_enabled: boolean;
  counter_channel_id: string | null;
  counter_format: string;

  voice_counter_enabled: boolean;
  voice_counter_channel_id: string | null;
  voice_counter_format: string;

  anniversary_enabled: boolean;
  anniversary_channel_id: string | null;
  anniversary_message: string;
  anniversary_title: string;
  anniversary_image_url: string;
  anniversary_footer_text: string;

  rejoin_message: string;
  rejoin_title: string;
  rejoin_image_url: string;
  rejoin_footer_text: string;
}

/** Payload PATCH-like : tous les champs sont optionnels (merge cote API). */
export type SaveWelcomeConfigParams = Partial<Omit<WelcomeConfig, "guild_id">>;
