"""Emetteur d'evenements JSON Lines sur stdout pour communication parent/enfant."""

import json
import sys
import time
from pathlib import Path


class Emitter:
    """Ecrit des lignes JSON sur stdout, une par evenement.

    Chaque ligne: {"event": "<type>", ...payload}
    Le parent Rust parse chaque ligne et relaie via app.emit().
    """

    def __init__(self, stop_flag_path: str | None = None) -> None:
        self.stop_flag = Path(stop_flag_path) if stop_flag_path else None
        self._last_batch_emit = 0.0

    def emit(self, event: str, **payload) -> None:
        payload["event"] = event
        try:
            sys.stdout.write(json.dumps(payload, ensure_ascii=False) + "\n")
            sys.stdout.flush()
        except Exception:
            pass

    def emit_batch_throttled(self, **payload) -> None:
        """Emet un event batch au maximum tous les 150 ms pour eviter de saturer."""
        now = time.monotonic()
        if now - self._last_batch_emit < 0.15:
            return
        self._last_batch_emit = now
        self.emit("batch", **payload)

    def should_stop(self) -> bool:
        if self.stop_flag is None:
            return False
        return self.stop_flag.exists()
