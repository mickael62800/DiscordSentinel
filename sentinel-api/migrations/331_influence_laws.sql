-- Jeu « Influence » — Phase 3 : cycle de loi (depot -> vote -> application).
-- La table influence_laws existe deja (migration 329). On ajoute le suivi du
-- message Discord (pour que le worker fasse editer le message a la cloture) et
-- un index de scan pour le worker « monde vivant ».

ALTER TABLE influence_laws ADD COLUMN IF NOT EXISTS channel_id TEXT;
ALTER TABLE influence_laws ADD COLUMN IF NOT EXISTS message_id TEXT;

-- Scan worker : lois en cours de vote dont l'echeance est passee.
CREATE INDEX IF NOT EXISTS idx_influence_laws_due
    ON influence_laws (expires_at) WHERE status = 'vote';
