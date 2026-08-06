// Envoi d'un message texte par le bot dans un salon.
//
// Rien n'est persisté : l'API dépose l'ordre sur le stream, le bot poste. Il
// n'y a donc ni liste, ni édition, ni suppression — un message envoyé
// appartient à Discord.

import { httpPost } from "@/api/http";

export interface SendResult {
  /// `queued`, pas `sent` : le bot n'a pas encore posté au moment de la
  /// réponse. Un salon interdit au bot échouera silencieusement côté Discord.
  queued: boolean;
}

/// Limite Discord, dupliquée ici pour afficher le compteur AVANT l'envoi. Le
/// serveur revalide : cette copie est un confort, pas la règle.
export const MAX_MESSAGE_LENGTH = 2000;

export const messagesService = {
  send(guildId: string, channelId: string, content: string): Promise<SendResult> {
    return httpPost(`/api/messages/${guildId}/${channelId}`, { content });
  },
};
