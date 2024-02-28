use rusqlite::{OptionalExtension, Savepoint, TransactionBehavior};

use crate::connection::program::Program;

use super::{Error, MigrationTaskStatus};

pub fn handle_migration_tasks<W>(conn: &mut libsql_sys::Connection<W>) -> Result<(), Error> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS __libsql_migration_tasks (
            job_id INTEGER PRIMARY KEY,
            status INTEGER,
            miration TEXT NOT NULL
    )",
        (),
    )?;

    // perform the pending jobs 1 by 1
    loop {
        let mut txn = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let Some((job_id, status, migration)) = txn
            .query_row(
                r#"SELECT * FROM __libsql_migration_tasks WHERE status = ? OR status = ? LIMIT 1"#,
                (
                    MigrationTaskStatus::Enqueued.encode_json(),
                    MigrationTaskStatus::Run.encode_json(),
                ),
                |row| {
                    let job_id = row.get::<_, i64>(0)?;
                    let status =
                        MigrationTaskStatus::decode_json(row.get_ref(1)?.as_str()?).unwrap();
                    let migration: Program =
                        serde_json::from_str(row.get_ref(2)?.as_str()?).unwrap();
                    Ok((job_id, status, migration))
                },
            )
            .optional()
            .unwrap()
        else {
            break;
        };

        let is_dry_run = match status {
            MigrationTaskStatus::Enqueued => {
                // TODO: force backup here
                // perform dry run
                true
            }
            MigrationTaskStatus::Run => false,
            _ => unreachable!(),
        };

        perform_migration(&mut txn, migration, is_dry_run, job_id);

        txn.commit().unwrap();
    }

    Ok(())
}

fn perform_migration(
    txn: &mut rusqlite::Transaction,
    migration: Program,
    dry_run: bool,
    job_id: i64,
) {
    let mut savepoint = txn.savepoint().unwrap();
    let status = match try_perform_migration(&mut savepoint, migration) {
        Ok(()) => {
            savepoint.commit().unwrap();
            if dry_run {
                MigrationTaskStatus::DryRunSuccess
            } else {
                MigrationTaskStatus::Success
            }
        }
        Err(e) => {
            savepoint.rollback().unwrap();
            drop(savepoint);
            if dry_run {
                MigrationTaskStatus::DryRunFailure {
                    error: e.to_string(),
                }
            } else {
                MigrationTaskStatus::Failure {
                    error: e.to_string(),
                }
            }
        }
    };

    txn.execute(
        "UPDATE __libsql_migration_tasks SET status = ? WHERE job_id = ?",
        (status.encode_json(), job_id),
    )
    .unwrap();
}

fn try_perform_migration(_savepoint: &mut Savepoint, _migration: Program) -> crate::Result<()> {
    todo!()
}
