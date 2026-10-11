use chrono::{DateTime, SubsecRound, Utc};

pub fn now() -> DateTime<Utc> {
    Utc::now().trunc_subsecs(0)
}
