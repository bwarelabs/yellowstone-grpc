use {
    crate::{
        metrics::{QUOTA_CHECKER_DURATION, QUOTA_CHECKER_RUNS, TEAMS_CAPPED, TEAMS_CHECKED},
        redis::refreshing_fallback_cache::RefreshingFallbackCache,
        user_connection::connection_manager::ConnectionManager,
    },
    log::{error, info},
    std::sync::Arc,
    time::OffsetDateTime,
    tokio::time::{interval, Duration},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct QuotaKey {
    year_month: String,
    team_id: String,
}

impl QuotaKey {
    fn to_suffix(&self) -> String {
        format!("{}:{}", self.year_month, self.team_id)
    }

    fn from_suffix(suffix: &str) -> Option<Self> {
        let mut parts = suffix.splitn(2, ':');
        let year_month = parts.next()?.to_string();
        let team_id = parts.next()?.to_string();
        Some(Self {
            year_month,
            team_id,
        })
    }
}


pub async fn start_redis_quota_checker(
    manager: Arc<ConnectionManager>,
    quota_cache: Arc<RefreshingFallbackCache<bool>>,
    check_interval: Duration,
    quota_check_batch_size: usize,
) {
    let mut ticker = interval(check_interval);

    loop {
        ticker.tick().await;

        let start = std::time::Instant::now();
        QUOTA_CHECKER_RUNS.inc();

        let teams = manager.list_active_teams();
        TEAMS_CHECKED.inc_by(teams.len() as u64);

        let now = OffsetDateTime::now_utc();
        let year_month = format!("{:04}-{:02}", now.year(), now.month() as u8);

        for team_chunk in teams.chunks(quota_check_batch_size) {
            let quota_keys: Vec<QuotaKey> = team_chunk
                .iter()
                .map(|team_id| QuotaKey {
                    year_month: year_month.clone(),
                    team_id: team_id.clone(),
                })
                .collect();

            let key_suffixes: Vec<String> = quota_keys.iter().map(QuotaKey::to_suffix).collect();

            let results = quota_cache.get_many_or_refresh(&key_suffixes).await;

            for (key_suffix, result) in results {
                let Some(quota_key) = QuotaKey::from_suffix(&key_suffix) else {
                    error!("Invalid quota key format: {}", key_suffix);
                    continue;
                };

                match result {
                    Ok(true) => {
                        TEAMS_CAPPED.inc();
                        info!(
                            "Team {} is capped, shutting down connection",
                            quota_key.team_id
                        );
                        manager.shutdown_client(&quota_key.team_id);
                    }
                    Ok(false) => {
                        // Not capped — do nothing
                    }
                    Err(e) => {
                        error!(
                            "Failed to check quota for team {}: {:?}",
                            quota_key.team_id, e
                        );
                    }
                }
            }
        }

        QUOTA_CHECKER_DURATION.observe(start.elapsed().as_secs_f64());
    }
}
