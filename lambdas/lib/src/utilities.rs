use anyhow::{Context, Result};
use chrono::{DateTime, Datelike, Days, FixedOffset, Utc, Weekday};


// JST: (2024-10-13, 2024-10)
pub fn get_date_month(timestamp: u64) -> Result<(String, String)> {
    let date_time = DateTime::from_timestamp(timestamp as i64, 0).context("Error converting from timestamp")?;
    let local =  date_time.with_timezone(&get_jst_timezone()?);
    let date = local.format("%Y-%m-%d").to_string();
    let month = local.format("%Y-%m").to_string();
    println!("date: {}, month: {}", date, month);
    Ok((date, month))
}

// JST Previous Weekday
pub fn get_previous_weekday() -> Result<String> {
    let date_time = Utc::now();
    let local =  date_time.with_timezone(&get_jst_timezone()?);
    let previous_day = if local.weekday() == Weekday::Mon {
        local.checked_sub_days(Days::new(3)).context("Error getting previous weekday")?
    } else {
        local.checked_sub_days(Days::new(1)).context("Error getting previous weekday")?
    };
    let day_string = previous_day.format("%Y-%m-%d").to_string();
    println!("day_string: {}", day_string);
    Ok(day_string)
}

// +09:00
fn get_jst_timezone() -> Result<FixedOffset> {
    FixedOffset::east_opt(9 * 3600).context("Error getting timezone")
}