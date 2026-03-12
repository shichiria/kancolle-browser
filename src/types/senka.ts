// Senka (戦果) related types

export interface SenkaSummary {
  total: number;
  exp_senka: number;
  eo_bonus: number;
  quest_bonus: number;
  monthly_exp_gain: number;
  tracking_active: boolean;
  next_checkpoint: string;
  checkpoint_crossed: boolean;
  eo_cutoff_active: boolean;
  quest_cutoff_active: boolean;
  confirmed_senka: number | null;
  confirmed_cutoff: string | null;
  is_confirmed_base: boolean;
}
