use {
    chrono::{Datelike, Utc},
    std::time::Duration,
};

/// Converts a relative duration assumed from now to an absolute UNIX timestamp measured in
/// seconds.
///
/// Uses an [`i64`] instead of an [`u64`] due to SQL having no proper unsigned integer type.
/// Converting wouldn't have any advantage then.
#[must_use]
pub fn relative_to_absolute(duration: Duration) -> i64 {
    let now = Utc::now();
    let duration =
        chrono::Duration::from_std(duration).expect("chrono bottlenecking for no reason");
    (now + duration).timestamp()
}

/// Converts a [`time::Date`] to a [`chrono::NaiveDate`], as they don't provide any direct
/// interconversion methods.
#[must_use]
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

    chrono::NaiveDate::from_isoywd_opt(iso_date.0, u32::from(iso_date.1), chrono_weekday).unwrap()
}

#[must_use]
pub fn time_to_chrono_datetime(time_datetime: time::PrimitiveDateTime) -> chrono::NaiveDateTime {
    time_to_chrono_date(time_datetime.date())
        .and_hms_opt(
            u32::from(time_datetime.hour()),
            u32::from(time_datetime.minute()),
            u32::from(time_datetime.second()),
        )
        .unwrap()
}

#[must_use]
pub fn format_date(date: chrono::NaiveDate) -> String {
    date.format("%A, %d.%m.%Y").to_string()
}

#[must_use]
pub fn figure_out_exact_deadline(
    deadline_weekday: u32,
    drive_date: chrono::NaiveDate,
) -> chrono::NaiveDateTime {
    // figure out how many days we need to backtrack
    let drive_weekday = drive_date.weekday().number_from_monday() - 1;

    let days_apart = if deadline_weekday < drive_weekday {
        // they're in the same week
        drive_weekday - deadline_weekday
    } else {
        // they're one week apart, so offset
        7 - (deadline_weekday - drive_weekday)
    };

    // ...and actually backtrack
    let deadline_date = drive_date - chrono::Days::new(u64::from(days_apart));

    deadline_date.and_hms_opt(23, 00, 00).unwrap()
}
