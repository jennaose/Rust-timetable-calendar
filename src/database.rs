use crate::TimetableEntry;
use postgres::{Client, NoTls, Error};

fn connect(conn_str: &str) -> Result<Client, Error> {
    Client::connect(conn_str, NoTls)
}

pub fn insert_entry(conn_str: &str, entry: &TimetableEntry) -> Result<(), Error> {
    let mut client = connect(conn_str)?;
    client.execute(
        "INSERT INTO timetable_entries (activity, time, day, notes) VALUES ($1, $2, $3, $4)",
        &[&entry.activity, &entry.time, &entry.day, &entry.notes],
    )?;
    Ok(())
}

pub fn load_entries(conn_str: &str) -> Result<Vec<TimetableEntry>, Error> {
    let mut client = connect(conn_str)?;
    let mut out = Vec::new();

    for row in client.query("SELECT activity, time, day, notes FROM timetable_entries ORDER BY id", &[])? {
        let activity: String = row.get(0);
        let time: String = row.get(1);
        let day: String = row.get(2);
        let notes: Option<String> = row.get(3);

        out.push(TimetableEntry {
            activity,
            time,
            day,
            notes: notes.unwrap_or_default(),
        });
    }

    Ok(out)
}

pub fn delete_entry(conn_str: &str, activity: &str, time: &str, day: &str) -> Result<(), Error> {
    let mut client = connect(conn_str)?;
    client.execute(
        "DELETE FROM timetable_entries WHERE activity = $1 AND time = $2 AND day = $3",
        &[&activity, &time, &day],
    )?;
    Ok(())
}