use serde::Deserialize;

/// A single entry in the ranking API response
#[derive(Debug, Deserialize, Clone)]
pub struct ApiRankingEntry {
    /// Encrypted position
    pub api_mxltvkpyuklh: Option<i64>,
    /// Encrypted admiral name
    pub api_mtjmdcwtvhdr: Option<String>,
    /// Encrypted senka rate
    pub api_wuhnhojjxmke: Option<f64>,
    /// Encrypted medal count
    pub api_itslcqtmrxtf: Option<i64>,
    /// Comment
    pub api_itbrdpdbkynm: Option<String>,
    #[serde(flatten)]
    _extra: serde_json::Value,
}

/// Response for api_req_ranking/mxltvkpyuklh
#[derive(Debug, Deserialize, Clone)]
pub struct ApiRankingResponse {
    #[allow(dead_code)] // kept for API schema completeness
    pub api_count: Option<i64>,
    pub api_list: Vec<ApiRankingEntry>,
}
