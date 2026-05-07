// Types Phase 10+ — analytics jeux casino (slot, wheel).

export interface SlotSpin {
  id: string;
  user_id: string;
  username: string;
  mise: number;
  symbols: string[];
  payout: number;
  multiplier: number;
  is_jackpot: boolean;
  is_free: boolean;
  created_at: string;
}

export interface SlotTopWinner {
  user_id: string;
  username: string;
  total_payout: number;
  jackpot_count: number;
  spin_count: number;
}

export interface JackpotPool {
  current_pool: number;
}

export interface WheelSpinLog {
  id: string;
  user_id: string;
  username: string;
  case_key: string;
  case_label: string;
  payout: number;
  created_at: string;
}

export interface WheelTopWinner {
  user_id: string;
  username: string;
  total_payout: number;
  spin_count: number;
}
