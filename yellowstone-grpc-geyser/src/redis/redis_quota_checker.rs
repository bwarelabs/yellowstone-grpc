use {
    crate::{
        redis::refreshing_fallback_cache::RefreshingFallbackCache,
        user_connection::connection_manager::ConnectionManager,
        metrics::{QUOTA_CHECKER_RUNS, TEAMS_CHECKED, TEAMS_CAPPED, QUOTA_CHECKER_DURATION},
    },
    std::sync::Arc,
    time::OffsetDateTime,
    tokio::time::{interval, Duration},
};

pub async fn start_redis_quota_checker(
    manager: Arc<ConnectionManager>,
    quota_cache: Arc<RefreshingFallbackCache<bool>>,
    check_interval: Duration,
) {
    let mut ticker = interval(check_interval);

    loop {
        ticker.tick().await;

        let start = std::time::Instant::now();
        QUOTA_CHECKER_RUNS.inc();

        let teams = manager.list_active_teams();
        TEAMS_CHECKED.inc_by(teams.len() as u64);

        for team_id in teams {
            let now = OffsetDateTime::now_utc();
            let year_month = format!("{:04}-{:02}", now.year(), now.month() as u8);
            let key_suffix = format!("{}:{}", year_month, team_id);

            match quota_cache.get_or_refresh(&key_suffix).await {
                Ok(true) => {
                    TEAMS_CAPPED.inc();
                    log::info!("Team {} is capped, shutting down connection", team_id);
                    manager.shutdown_client(&team_id);
                }
                Ok(false) => {
                    // Team is fine, do nothing
                }
                Err(e) => {
                    log::error!("Failed to check quota for team {}: {:?}", team_id, e);
                }
            }
        }

        QUOTA_CHECKER_DURATION.observe(start.elapsed().as_secs_f64());
    }
}
