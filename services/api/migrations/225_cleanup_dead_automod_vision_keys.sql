-- Cleanup automod-bot : 9 cles vision_* sont vestigials.
-- Mig 156 (fusion image-bot -> automod-bot) avait copie ces cles dans
-- le schema mais le code (bot, API, worker) ne les lit nulle part.
--
-- Dead keys :
--   vision_max_image_size_mb, vision_scan_embeds
--   vision_queue_enabled, vision_queue_max_retries
--   vision_hash_cache_enabled, vision_hash_cache_ttl_secs
--   vision_channel_thresholds
--   vision_auto_delete_nsfw, vision_auto_delete_illicit
--
-- A reimplementer si on cable ces features. Aujourd'hui :
--   - vision_enabled (toggle global) ✅ lu par message_handler
--   - vision_threshold ✅ lu par analyze_image_service
--   - autres : ignores

UPDATE bot_definitions
   SET config_schema = (
       SELECT jsonb_agg(entry)
         FROM jsonb_array_elements(config_schema) AS entry
        WHERE entry->>'key' NOT IN (
            'vision_max_image_size_mb',
            'vision_scan_embeds',
            'vision_queue_enabled',
            'vision_queue_max_retries',
            'vision_hash_cache_enabled',
            'vision_hash_cache_ttl_secs',
            'vision_channel_thresholds',
            'vision_auto_delete_nsfw',
            'vision_auto_delete_illicit'
        )
   )
 WHERE bot_name = 'automod-bot';

DELETE FROM bot_guild_config
 WHERE bot_name = 'automod-bot'
   AND config_key IN (
       'vision_max_image_size_mb',
       'vision_scan_embeds',
       'vision_queue_enabled',
       'vision_queue_max_retries',
       'vision_hash_cache_enabled',
       'vision_hash_cache_ttl_secs',
       'vision_channel_thresholds',
       'vision_auto_delete_nsfw',
       'vision_auto_delete_illicit'
   );
