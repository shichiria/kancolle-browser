// Common types shared across multiple domains

export interface ConditionResult {
  condition: string;
  satisfied: boolean;
  current_value: string;
  required_value: string;
}

export type TabId = "homeport" | "battle" | "improvement" | "ships" | "equips" | "options";

export interface DriveStatus {
  authenticated: boolean;
  email?: string;
  syncing: boolean;
  last_sync?: string;
  error?: string;
}
