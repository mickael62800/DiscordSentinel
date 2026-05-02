-- Ajoute xp_channel_multipliers et xp_role_multipliers au config_schema
-- du progression-bot pour qu'ils apparaissent dans la page Composants
-- (apps/web /component-config) et soient editables via l'UI generique.
--
-- Format attendu (consume cote bot dans
-- bots/sentinel-bot/src/modules/progression/multipliers.rs) :
--   ID:multiplicateur     # un mapping par ligne
-- ex :
--   123456789012345678:2.0
--   987654321098765432:0.5
--
-- Le frontend rend un <textarea> automatiquement pour les cles type=text
-- finissant par "_multipliers" (cf isMultilineKey dans
-- ComponentConfigPage.vue, ouvert pour cette migration).

UPDATE bot_definitions SET config_schema = config_schema::jsonb || '[
    {"key": "xp_channel_multipliers", "label": "Multiplicateurs XP par salon (ID:mult par ligne)", "type": "text", "required": false},
    {"key": "xp_role_multipliers", "label": "Multiplicateurs XP par role (ID:mult par ligne)", "type": "text", "required": false}
]'::jsonb
WHERE bot_name = 'progression-bot';
