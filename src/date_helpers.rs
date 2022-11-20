use {chrono::Utc, std::time::Duration};

/// Converts a relative duration assumed from now to an absolute UNIX timestamp measured in
/// seconds.
///
/// Uses an [`i64`] instead of an [`u64`] due to SQL having no proper unsigned integer type.
/// Converting wouldn't have any advantage then.
pub fn relative_to_absolute(duration: Duration) -> i64 {
    let now = Utc::now();
    let duration =
        chrono::Duration::from_std(duration).expect("chrono bottlenecking for no reason");
    (now + duration).timestamp()
}

/// Converts a [`time::Date`] to a [`chrono::NaiveDate`], as they don't provide any direct
/// interconversion methods.
pub fn time_to_chrono_date(time_date: time::Date) -> chrono::NaiveDate {
    let iso_date = time_date.to_iso_week_date();
    let chrono_weekday = match time_date.weekday() {
        time::Weekday::Monday => chrono::Weekday::Mon,
        time::Weekday::Tuesday => chrono::Weekday::Tue,
        time::Weekday::Wednesday => chrono::Weekday::Wed,
        time::Weekday::Thursday => chrono::Weekday::Thu,
        time::Weekday::Friday => chrono::Weekday::Fri,
        time::Weekday::Saturday => chrono::Weekday::Sat,
        time::Weekday::Sunday => chrono::Weekday::Sun,
    };

    chrono::NaiveDate::from_isoywd(iso_date.0, u32::from(iso_date.1), chrono_weekday)
}

pub fn format_date(date: chrono::NaiveDate) -> String {
    date.format("%A, %d.%m.%Y").to_string()
}
