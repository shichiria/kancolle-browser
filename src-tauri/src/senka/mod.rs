use chrono::{Datelike, FixedOffset, TimeZone, Timelike, Utc};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::path::Path;

// =============================================================================
// Ranking API decryption (ported from EO SenkaLeaderboardViewModel.cs)
// =============================================================================

/// Key table for ranking decryption (13 values, indexed by rank % 13)
const POSSIBLE_RANK: [i64; 13] = [
    8931, 1201, 1156, 5061, 4569, 4732, 3779, 4568, 5695, 4619, 4912, 5669, 6586,
];

/// A decrypted ranking entry
#[derive(Debug, Clone, Serialize)]
pub struct RankingEntry {
    pub position: i32,
    pub admiral_name: String,
    pub senka: i64,
    pub medal_count: i32,
    pub comment: String,
}

/// Check if a candidate user_key produces an integer senka >= 0
fn check_rate(key: i64, user_key: i64, rate: f64) -> bool {
    let points = rate / (key as f64) / (user_key as f64) - 91.0;
    points >= 0.0 && (points - points.floor()).abs() < 1e-6
}

/// Decrypt ranking entries from the raw API response.
/// Returns (decoded entries, user's own senka if found).
pub fn decrypt_ranking(json_str: &str, admiral_name: &str) -> (Vec<RankingEntry>, Option<i64>) {
    // Parse the ranking API response
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(json_str);
    let root = match parsed {
        Ok(v) => v,
        Err(e) => {
            warn!("Senka: failed to parse ranking JSON: {}", e);
            return (vec![], None);
        }
    };

    // Navigate to api_data.api_list
    let api_list = root
        .get("api_data")
        .and_then(|d| d.get("api_list"))
        .and_then(|l| l.as_array());

    let entries = match api_list {
        Some(arr) => arr,
        None => {
            warn!("Senka: ranking response has no api_data.api_list");
            return (vec![], None);
        }
    };

    // Phase 1: Narrow down possible user keys using all entries
    let mut possible_user_keys: Vec<i64> = Vec::new();

    for entry in entries {
        let position = entry
            .get("api_mxltvkpyuklh")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let rate = entry
            .get("api_wuhnhojjxmke")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        if position <= 0 || rate <= 0.0 {
            continue;
        }

        let key = POSSIBLE_RANK[(position % 13) as usize];

        if possible_user_keys.is_empty() {
            // First entry: try all keys 10-99
            for uk in 10..100 {
                if check_rate(key, uk, rate) {
                    possible_user_keys.push(uk);
                }
            }
        } else {
            // Subsequent entries: filter down
            possible_user_keys.retain(|&uk| check_rate(key, uk, rate));
        }
    }

    if possible_user_keys.is_empty() {
        warn!("Senka: could not determine user key for ranking decryption");
        return (vec![], None);
    }

    let user_key = *possible_user_keys.last().unwrap();
    info!(
        "Senka: ranking user_key determined: {} (from {} candidates)",
        user_key,
        possible_user_keys.len()
    );

    // Phase 2: Decrypt all entries
    let mut decoded = Vec::new();
    let mut own_senka = None;

    for entry in entries {
        let position = entry
            .get("api_mxltvkpyuklh")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        let name = entry
            .get("api_mtjmdcwtvhdr")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let rate = entry
            .get("api_wuhnhojjxmke")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let medal_enc = entry
            .get("api_itslcqtmrxtf")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let comment = entry
            .get("api_itbrdpdbkynm")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if position <= 0 {
            continue;
        }

        let key = POSSIBLE_RANK[(position as i64 % 13) as usize];
        let senka = (rate / (key as f64) / (user_key as f64)).floor() as i64 - 91;
        let medal_count = (medal_enc / (key + 1853)) as i32 - 157;

        let re = RankingEntry {
            position,
            admiral_name: name.clone(),
            senka: senka.max(0),
            medal_count: medal_count.max(0),
            comment,
        };

        // Check if this is our admiral
        if name == admiral_name {
            info!(
                "Senka: found own entry at rank {} with senka {}",
                position, senka
            );
            own_senka = Some(senka.max(0));
        }

        decoded.push(re);
    }

    info!(
        "Senka: decoded {} ranking entries, own senka: {:?}",
        decoded.len(),
        own_senka
    );
    (decoded, own_senka)
}

/// JST offset (+09:00)
const JST_OFFSET: i32 = 9 * 3600;

fn jst() -> FixedOffset {
    FixedOffset::east_opt(JST_OFFSET).unwrap()
}

fn now_jst() -> chrono::DateTime<FixedOffset> {
    Utc::now().with_timezone(&jst())
}

/// Ranking point item ID -> bonus value mapping (from clearitemget api_bounus api_type:18)
pub fn senka_item_bonus(api_id: i64) -> i64 {
    match api_id {
        895 => 440,
        896 => 50,
        897 => 11,
        898 => 800,
        900 => 200,
        901 => 350,
        902 => 180,
        903 => 300,
        904 => 165,
        905 => 175,
        907 => 210,
        908 => 215,
        909 => 330,
        910 => 400,
        911 => 250,
        912 => 315,
        913 => 340,
        914 => 160,
        _ => {
            warn!("Unknown senka item ID: {}, treating as count-based", api_id);
            0
        }
    }
}

/// EO map -> ranking bonus points
pub fn eo_bonus_for_map(area: i32, map: i32) -> i64 {
    match (area, map) {
        (1, 5) => 75,
        (1, 6) => 75,
        (2, 5) => 100,
        (3, 5) => 150,
        (4, 5) => 180,
        (5, 5) => 200,
        (6, 5) => 250,
        (7, 5) => 170,
        _ => 0,
    }
}

/// A log entry for senka tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SenkaLogEntry {
    pub timestamp: String,
    #[serde(rename = "type")]
    pub entry_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exp_gain: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bonus: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Persistent senka tracking data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SenkaData {
    /// Year-month string e.g. "2026-03"
    pub month: String,
    /// HQ experience at month start (first port response after monthly boundary)
    pub month_start_exp: Option<i64>,
    /// Last known cumulative HQ experience
    pub last_exp: Option<i64>,
    /// EO bonus points total for this month
    pub eo_bonus: i64,
    /// Quest bonus points total for this month
    pub quest_bonus: i64,
    /// ISO8601 timestamp of last checkpoint (3:00 or 15:00 JST)
    pub last_checkpoint: Option<String>,

    // --- Confirmed senka from ranking page ---
    /// Confirmed senka value from ranking page decryption
    #[serde(default)]
    pub confirmed_senka: Option<i64>,
    /// Data cutoff time (02:00 or 14:00 JST) — the ranking value includes activity up to this time.
    /// Gains after this time need to be added to confirmed_senka.
    #[serde(default)]
    pub confirmed_cutoff: Option<String>,

    /// Log entries (exp gains, EO, quest bonuses with timestamps for time-based filtering)
    #[serde(default)]
    pub entries: Vec<SenkaLogEntry>,
}

impl Default for SenkaData {
    fn default() -> Self {
        let now = now_jst();
        Self {
            month: current_ranking_month(&now),
            month_start_exp: None,
            last_exp: None,
            eo_bonus: 0,
            quest_bonus: 0,
            last_checkpoint: None,
            confirmed_senka: None,
            confirmed_cutoff: None,
            entries: Vec::new(),
        }
    }
}

/// Senka summary sent to frontend
#[derive(Debug, Clone, Serialize)]
pub struct SenkaSummary {
    pub total: f64,
    /// Experience-based senka delta (after cutoff if confirmed, or full month if not)
    pub exp_senka: f64,
    /// EO bonus delta (after cutoff if confirmed, or full month if not)
    pub eo_bonus: i64,
    /// Quest bonus delta (after cutoff if confirmed, or full month if not)
    pub quest_bonus: i64,
    /// Exp gain delta (after cutoff if confirmed, or full month if not)
    pub monthly_exp_gain: i64,
    pub tracking_active: bool,
    pub next_checkpoint: String,
    pub checkpoint_crossed: bool,
    pub eo_cutoff_active: bool,
    pub quest_cutoff_active: bool,
    /// Confirmed senka from ranking page (None = not yet confirmed)
    pub confirmed_senka: Option<i64>,
    /// Data cutoff time (02:00 or 14:00 JST) that the confirmed senka reflects
    pub confirmed_cutoff: Option<String>,
    /// Whether total is based on confirmed ranking (true) or estimated (false)
    pub is_confirmed_base: bool,
}

/// Core senka tracker
#[derive(Debug)]
pub struct SenkaTracker {
    pub data: SenkaData,
    path: std::path::PathBuf,
}

impl SenkaTracker {
    pub fn new(data_dir: &Path) -> Self {
        let path = data_dir.join("sync").join("senka_log.json");
        let data = load_senka_data(&path);
        Self { data, path }
    }

    /// Monthly HQ experience gain (absolute diff from month_start_exp)
    fn monthly_exp_gain(&self) -> i64 {
        match (self.data.month_start_exp, self.data.last_exp) {
            (Some(start), Some(last)) => (last - start).max(0),
            _ => 0,
        }
    }

    /// Whether we have a confirmed senka base from the ranking page
    fn has_confirmed_base(&self) -> bool {
        self.data.confirmed_senka.is_some() && self.data.confirmed_cutoff.is_some()
    }

    /// Sum exp/eo/quest gains from entries after the given cutoff time.
    /// Returns (exp_gained, eo_bonus, quest_bonus).
    fn gains_after_cutoff(&self, cutoff: &str) -> (i64, i64, i64) {
        let mut exp_total = 0i64;
        let mut eo_total = 0i64;
        let mut quest_total = 0i64;

        for entry in &self.data.entries {
            if entry.timestamp.as_str() <= cutoff {
                continue;
            }
            match entry.entry_type.as_str() {
                "exp" => {
                    exp_total += entry.exp_gain.unwrap_or(0);
                }
                "eo" => {
                    eo_total += entry.bonus.unwrap_or(0);
                }
                "quest" => {
                    quest_total += entry.bonus.unwrap_or(0);
                }
                _ => {}
            }
        }
        (exp_total, eo_total, quest_total)
    }

    /// Calculate total senka
    pub fn total_senka(&self) -> f64 {
        if let (Some(base), Some(cutoff)) =
            (self.data.confirmed_senka, self.data.confirmed_cutoff.as_deref())
        {
            let (exp, eo, quest) = self.gains_after_cutoff(cutoff);
            base as f64 + exp as f64 * 7.0 / 10000.0 + eo as f64 + quest as f64
        } else {
            // Fallback: estimate from month start exp
            self.monthly_exp_gain() as f64 * 7.0 / 10000.0
                + self.data.eo_bonus as f64
                + self.data.quest_bonus as f64
        }
    }

    /// Get summary for frontend
    pub fn summary(&self) -> SenkaSummary {
        self.summary_with_checkpoint(false)
    }

    pub fn summary_with_checkpoint(&self, checkpoint_crossed: bool) -> SenkaSummary {
        let now = now_jst();
        let is_confirmed = self.has_confirmed_base();

        let (exp_senka, eo_bonus, quest_bonus, exp_gain) = if let Some(cutoff) =
            self.data.confirmed_cutoff.as_deref()
        {
            if is_confirmed {
                let (exp, eo, quest) = self.gains_after_cutoff(cutoff);
                (exp as f64 * 7.0 / 10000.0, eo, quest, exp)
            } else {
                // Has cutoff but no confirmed senka — shouldn't happen, fallback
                let mg = self.monthly_exp_gain();
                (mg as f64 * 7.0 / 10000.0, self.data.eo_bonus, self.data.quest_bonus, mg)
            }
        } else {
            let mg = self.monthly_exp_gain();
            (mg as f64 * 7.0 / 10000.0, self.data.eo_bonus, self.data.quest_bonus, mg)
        };

        SenkaSummary {
            total: self.total_senka(),
            exp_senka,
            eo_bonus,
            quest_bonus,
            monthly_exp_gain: exp_gain,
            tracking_active: self.data.month_start_exp.is_some(),
            next_checkpoint: next_checkpoint_iso(),
            checkpoint_crossed,
            eo_cutoff_active: is_eo_cutoff(&now),
            quest_cutoff_active: is_quest_cutoff(&now),
            confirmed_senka: self.data.confirmed_senka,
            confirmed_cutoff: self.data.confirmed_cutoff.clone(),
            is_confirmed_base: is_confirmed,
        }
    }

    /// Update with new HQ experience from api_port/port.
    /// Returns (changed, checkpoint_crossed).
    pub fn update_experience(&mut self, current_exp: i64) -> (bool, bool) {
        let now = now_jst();
        let current_month = current_ranking_month(&now);

        // Month boundary check
        if self.data.month != current_month {
            info!(
                "Senka: month boundary crossed {} -> {}",
                self.data.month, current_month
            );
            self.reset_month(&current_month, current_exp);
            return (true, false);
        }

        // First port of the month — set month_start_exp
        if self.data.month_start_exp.is_none() {
            info!("Senka: setting month_start_exp = {}", current_exp);
            self.data.month_start_exp = Some(current_exp);
            self.data.last_exp = Some(current_exp);
            self.save();
            return (true, false);
        }

        let prev_exp = self.data.last_exp.unwrap_or(current_exp);
        let delta = current_exp - prev_exp;
        self.data.last_exp = Some(current_exp);

        let changed = delta > 0;
        if changed {
            info!(
                "Senka: port exp update +{}, monthly gain: {}, senka: {:.1}",
                delta,
                self.monthly_exp_gain(),
                self.total_senka()
            );
        }

        // Checkpoint detection (3:00 / 15:00 JST)
        let checkpoint_crossed = self.check_checkpoint(&now);

        if changed || checkpoint_crossed {
            self.save();
        }

        (changed, checkpoint_crossed)
    }

    /// Record HQ exp gained from a single battle (api_get_exp from battleresult).
    /// Each battle is recorded individually with its timestamp for precise time-based filtering.
    pub fn add_battle_exp(&mut self, exp: i64, map_display: &str) {
        if exp <= 0 {
            return;
        }
        let now = now_jst();
        self.data.entries.push(SenkaLogEntry {
            timestamp: now.to_rfc3339(),
            entry_type: "exp".to_string(),
            exp_gain: Some(exp),
            bonus: None,
            detail: Some(format!("{} 提督exp+{}", map_display, exp)),
        });
        info!(
            "Senka: battle exp +{} at {}, senka: {:.1}",
            exp,
            map_display,
            self.total_senka()
        );
        self.save();
    }

    /// Add EO ranking bonus
    pub fn add_eo_bonus(&mut self, bonus: i64, map_display: &str) {
        if bonus <= 0 {
            return;
        }

        let now = now_jst();
        if is_eo_cutoff(&now) {
            warn!(
                "Senka: EO bonus {} from {} ignored (past month-end 22:00 JST cutoff)",
                bonus, map_display
            );
            self.data.entries.push(SenkaLogEntry {
                timestamp: now.to_rfc3339(),
                entry_type: "eo_cutoff".to_string(),
                exp_gain: None,
                bonus: Some(bonus),
                detail: Some(format!("{} EOクリア (22:00以降のため戦果無効)", map_display)),
            });
            self.save();
            return;
        }

        self.data.eo_bonus += bonus;
        info!(
            "Senka: EO bonus +{} from {}, total EO: {}, senka: {:.1}",
            bonus,
            map_display,
            self.data.eo_bonus,
            self.total_senka()
        );
        self.data.entries.push(SenkaLogEntry {
            timestamp: now.to_rfc3339(),
            entry_type: "eo".to_string(),
            exp_gain: None,
            bonus: Some(bonus),
            detail: Some(format!("{} EOクリア +{}", map_display, bonus)),
        });
        self.save();
    }

    /// Confirm senka from ranking page decryption.
    /// Determines the data cutoff time (02:00 or 14:00 JST) that the ranking reflects,
    /// then stores it so future gains are summed from entries after that cutoff.
    pub fn confirm_ranking(&mut self, senka: i64) {
        let now = now_jst();
        let cutoff = ranking_data_cutoff(&now);
        let cutoff_str = cutoff.to_rfc3339();

        info!(
            "Senka: ranking confirmed: senka={}, cutoff={}, exp={:?}",
            senka, cutoff_str, self.data.last_exp
        );

        self.data.confirmed_senka = Some(senka);
        self.data.confirmed_cutoff = Some(cutoff_str.clone());

        self.data.entries.push(SenkaLogEntry {
            timestamp: now.to_rfc3339(),
            entry_type: "ranking_confirmed".to_string(),
            exp_gain: None,
            bonus: Some(senka),
            detail: Some(format!(
                "ランキング確認 戦果: {} (データ反映: {}まで)",
                senka,
                cutoff.format("%H:%M")
            )),
        });
        self.save();
    }

    /// Add quest ranking bonus from clearitemget api_type:18
    pub fn add_quest_bonus(&mut self, bonus: i64, quest_id: i32) {
        if bonus <= 0 {
            return;
        }

        let now = now_jst();
        let is_late = is_quest_cutoff(&now);

        self.data.quest_bonus += bonus;
        info!(
            "Senka: quest bonus +{} from quest {}{}, total quest: {}, senka: {:.1}",
            bonus,
            quest_id,
            if is_late { " (14:00以降・翌月扱い)" } else { "" },
            self.data.quest_bonus,
            self.total_senka()
        );

        let detail = if is_late {
            format!("任務{} 戦果+{} (14:00以降のため翌月扱い)", quest_id, bonus)
        } else {
            format!("任務{} 戦果+{}", quest_id, bonus)
        };

        self.data.entries.push(SenkaLogEntry {
            timestamp: now.to_rfc3339(),
            entry_type: if is_late {
                "quest_late".to_string()
            } else {
                "quest".to_string()
            },
            exp_gain: None,
            bonus: Some(bonus),
            detail: Some(detail),
        });
        self.save();
    }

    /// Check if a checkpoint (3:00/15:00 JST) was crossed since last check
    fn check_checkpoint(&mut self, now: &chrono::DateTime<FixedOffset>) -> bool {
        let last = self
            .data
            .last_checkpoint
            .as_ref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok());

        let checkpoints = get_recent_checkpoints(now);

        for cp in &checkpoints {
            let crossed = match &last {
                Some(last_cp) => cp > last_cp && cp <= now,
                None => cp <= now,
            };
            if crossed {
                info!("Senka: checkpoint crossed at {}", cp.to_rfc3339());
                self.data.last_checkpoint = Some(cp.to_rfc3339());
                self.data.entries.push(SenkaLogEntry {
                    timestamp: cp.to_rfc3339(),
                    entry_type: "checkpoint".to_string(),
                    exp_gain: None,
                    bonus: None,
                    detail: Some(format!(
                        "ランキング更新 推計戦果: {:.1}",
                        self.total_senka()
                    )),
                });
                return true;
            }
        }
        false
    }

    /// Reset for new month
    fn reset_month(&mut self, new_month: &str, current_exp: i64) {
        info!(
            "Senka: resetting for new month {} (prev senka: {:.1})",
            new_month,
            self.total_senka()
        );
        self.data = SenkaData {
            month: new_month.to_string(),
            month_start_exp: Some(current_exp),
            last_exp: Some(current_exp),
            eo_bonus: 0,
            quest_bonus: 0,
            last_checkpoint: None,
            confirmed_senka: None,
            confirmed_cutoff: None,
            entries: Vec::new(),
        };
        self.save();
    }

    fn save(&self) {
        save_senka_data(&self.path, &self.data);
    }

    /// Sync notifier path for Google Drive
    pub fn sync_path() -> &'static str {
        "senka_log.json"
    }
}

impl Default for SenkaTracker {
    fn default() -> Self {
        Self {
            data: SenkaData::default(),
            path: std::path::PathBuf::new(),
        }
    }
}

/// Determine the "ranking month" at a given JST time.
/// Experience after 22:00 on the last day counts for the NEXT month.
fn current_ranking_month(now: &chrono::DateTime<FixedOffset>) -> String {
    let last_day = last_day_of_month(now.year(), now.month());
    if now.day() == last_day && now.hour() >= 22 {
        // After 22:00 on last day → next month
        let next = if now.month() == 12 {
            format!("{}-01", now.year() + 1)
        } else {
            format!("{}-{:02}", now.year(), now.month() + 1)
        };
        return next;
    }
    format!("{}-{:02}", now.year(), now.month())
}

fn load_senka_data(path: &Path) -> SenkaData {
    match std::fs::read_to_string(path) {
        Ok(contents) => match serde_json::from_str::<SenkaData>(&contents) {
            Ok(data) => {
                let now = now_jst();
                let current_month = current_ranking_month(&now);
                if data.month == current_month {
                    info!("Senka: loaded data for {}", current_month);
                    data
                } else {
                    info!(
                        "Senka: saved data for {} but current month is {}, starting fresh",
                        data.month, current_month
                    );
                    SenkaData::default()
                }
            }
            Err(e) => {
                warn!("Senka: failed to parse senka_log.json: {}", e);
                SenkaData::default()
            }
        },
        Err(_) => {
            info!("Senka: no existing senka_log.json, starting fresh");
            SenkaData::default()
        }
    }
}

fn save_senka_data(path: &Path, data: &SenkaData) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match serde_json::to_string_pretty(data) {
        Ok(json) => {
            if let Err(e) = std::fs::write(path, json) {
                warn!("Senka: failed to write senka_log.json: {}", e);
            }
        }
        Err(e) => warn!("Senka: failed to serialize senka data: {}", e),
    }
}

fn get_recent_checkpoints(
    now: &chrono::DateTime<FixedOffset>,
) -> Vec<chrono::DateTime<FixedOffset>> {
    let mut checkpoints = Vec::new();

    let today_3 = jst()
        .with_ymd_and_hms(now.year(), now.month(), now.day(), 3, 0, 0)
        .single();
    let today_15 = jst()
        .with_ymd_and_hms(now.year(), now.month(), now.day(), 15, 0, 0)
        .single();

    if let Some(cp) = today_3 {
        checkpoints.push(cp);
    }
    if let Some(cp) = today_15 {
        checkpoints.push(cp);
    }

    // Yesterday's 15:00 for midnight crossing
    let yesterday = *now - chrono::Duration::days(1);
    if let Some(cp) = jst()
        .with_ymd_and_hms(
            yesterday.year(),
            yesterday.month(),
            yesterday.day(),
            15,
            0,
            0,
        )
        .single()
    {
        checkpoints.push(cp);
    }

    checkpoints.sort();
    checkpoints
}

fn next_checkpoint_iso() -> String {
    let now = now_jst();
    let today_3 = jst()
        .with_ymd_and_hms(now.year(), now.month(), now.day(), 3, 0, 0)
        .single();
    let today_15 = jst()
        .with_ymd_and_hms(now.year(), now.month(), now.day(), 15, 0, 0)
        .single();

    if let Some(cp) = today_3 {
        if now < cp {
            return cp.to_rfc3339();
        }
    }
    if let Some(cp) = today_15 {
        if now < cp {
            return cp.to_rfc3339();
        }
    }

    let tomorrow = now + chrono::Duration::days(1);
    jst()
        .with_ymd_and_hms(tomorrow.year(), tomorrow.month(), tomorrow.day(), 3, 0, 0)
        .single()
        .map(|cp| cp.to_rfc3339())
        .unwrap_or_default()
}

/// Determine the data cutoff time for the most recent ranking update.
/// Ranking at 03:00 reflects activity up to 02:00.
/// Ranking at 15:00 reflects activity up to 14:00.
/// Returns the cutoff time (02:00 or 14:00 JST).
fn ranking_data_cutoff(now: &chrono::DateTime<FixedOffset>) -> chrono::DateTime<FixedOffset> {
    if now.hour() >= 15 {
        // After 15:00 → ranking shows data up to today 14:00
        jst()
            .with_ymd_and_hms(now.year(), now.month(), now.day(), 14, 0, 0)
            .single()
            .unwrap()
    } else if now.hour() >= 3 {
        // After 03:00, before 15:00 → ranking shows data up to today 02:00
        jst()
            .with_ymd_and_hms(now.year(), now.month(), now.day(), 2, 0, 0)
            .single()
            .unwrap()
    } else {
        // Before 03:00 → ranking shows data up to yesterday 14:00
        let yesterday = *now - chrono::Duration::days(1);
        jst()
            .with_ymd_and_hms(yesterday.year(), yesterday.month(), yesterday.day(), 14, 0, 0)
            .single()
            .unwrap()
    }
}

fn is_eo_cutoff(now: &chrono::DateTime<FixedOffset>) -> bool {
    let last_day = last_day_of_month(now.year(), now.month());
    now.day() == last_day && now.hour() >= 22
}

fn is_quest_cutoff(now: &chrono::DateTime<FixedOffset>) -> bool {
    let last_day = last_day_of_month(now.year(), now.month());
    now.day() == last_day && now.hour() >= 14
}

fn last_day_of_month(year: i32, month: u32) -> u32 {
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    chrono::NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .and_then(|d| d.pred_opt())
        .map(|d| d.day())
        .unwrap_or(28)
}
