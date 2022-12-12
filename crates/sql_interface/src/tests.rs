use super::sql_struct::{NotEnoughValues, SqlStruct};
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use time::{macros::datetime, OffsetDateTime as DateTime};

#[derive(Debug, PartialEq, Eq)]
pub struct Drive {
    pub id: i64,
    pub date: DateTime,
    pub deadline: Option<DateTime>,
}

impl SqlStruct for Drive {
    fn required_tables() -> Vec<&'static str> {
        vec!["drive"]
    }

    fn select_exprs() -> Vec<&'static str> {
        vec!["drive.drive_id", "drive.drivedate", "drive.deadline"]
    }

    fn from_row<'a>(mut row: impl Iterator<Item = ValueRef<'a>>) -> FromSqlResult<Self> {
        Ok(Self {
            id: FromSql::column_result(
                row.next()
                    .ok_or_else(|| FromSqlError::Other(Box::new(NotEnoughValues)))?,
            )?,
            date: FromSql::column_result(
                row.next()
                    .ok_or_else(|| FromSqlError::Other(Box::new(NotEnoughValues)))?,
            )?,
            deadline: FromSql::column_result(
                row.next()
                    .ok_or_else(|| FromSqlError::Other(Box::new(NotEnoughValues)))?,
            )?,
        })
    }
}

#[test]
fn drive_roundtrip() {
    let first = Drive {
        id: 0,
        date: datetime!(2022-12-14 20:00:00 UTC),
        deadline: Some(datetime!(2022-12-12 16:00:00 UTC)),
    };
    let second = Drive {
        id: 1,
        date: datetime!(2022-12-17 19:30:00 UTC),
        deadline: None,
    };

    let row: Vec<ToSqlOutput> = vec![
        first.id.to_sql().unwrap(),
        first.date.to_sql().unwrap(),
        first.deadline.to_sql().unwrap(),
        second.id.to_sql().unwrap(),
        second.date.to_sql().unwrap(),
        second.deadline.to_sql().unwrap(),
    ];

    let row: Vec<_> = row
        .iter()
        .map(|sql_output| match sql_output {
            ToSqlOutput::Owned(value) => value.into(),
            ToSqlOutput::Borrowed(value) => *value,
            _ => unreachable!(),
        })
        .collect();

    let (produced_first, produced_second): (Drive, Drive) =
        SqlStruct::from_row(row.clone().into_iter()).unwrap();

    assert_eq!(produced_first, first);
    assert_eq!(produced_second, second);
}
