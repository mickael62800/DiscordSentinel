-- 007_cartes_de_session_partout.sql
--
-- Les cartes de session vocale couvrent desormais TOUS les vocaux permanents
-- par defaut, plus seulement ceux enumeres a la main.
--
-- Le champ `observed_voice_channels` ne fait plus l'inscription mais la
-- RESTRICTION : vide, tout est suivi ; renseigne, seuls les salons cites le
-- sont. Sa description disait l'inverse et devient trompeuse.
--
-- Contexte : ce champ est de type `voice_list`, un type qu'aucune section du
-- formulaire ne connaissait — il n'apparaissait donc PAS dans l'interface.
-- Personne ne pouvait y inscrire de salon, et aucun vocal permanent n'avait
-- de carte de session, sans que rien ne l'explique.

UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(
        CASE WHEN elem ->> 'key' = 'observed_voice_channels'
             THEN elem
                  || jsonb_build_object(
                       'label', 'Restreindre les cartes de session a ces vocaux',
                       'description',
                       'Vide = TOUS les vocaux permanents recoivent une carte de session, ouverte a la premiere arrivee et fermee quand le salon se vide. Renseigne, seuls les salons cites en recoivent une. Le salon AFK est toujours exclu.')
             ELSE elem END
        ORDER BY ord
    )
    FROM jsonb_array_elements(config_schema) WITH ORDINALITY AS t(elem, ord)
)
WHERE bot_name = 'voice-bot'
  AND config_schema @> '[{"key": "observed_voice_channels"}]'::jsonb;
