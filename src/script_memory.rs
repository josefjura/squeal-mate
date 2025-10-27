use crate::{infrastructure::get_script_database, entries::EntryStatus};
use color_eyre::eyre::{self};
use rusqlite::{named_params, Connection};
use std::path::PathBuf;

#[cfg(test)]
use std::sync::atomic::{AtomicU32, Ordering};

pub struct ScriptDatabaseRecord {
    crc: u32,
    result: bool,
}

#[derive(Clone, Debug)]
pub struct ScriptDatabase {
    db_name: PathBuf,
}

impl ScriptDatabase {
    pub async fn new() -> eyre::Result<Self> {
        let filename = get_script_database();
        let conn = Connection::open(filename.clone())?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS scripts (							
							name  TEXT NOT NULL PRIMARY KEY,
							result INTEGER NOT NULL,						
							crc	 	INTEGER NOT NULL
					)",
            (), // empty list of parameters.
        )?;
        Ok(ScriptDatabase { db_name: filename })
    }

    pub fn insert(&self, file: String, crc: u32, result: bool) -> eyre::Result<()> {
        let conn = Connection::open(self.db_name.clone())?;
        // Prepare the statement and insert the records
        let mut stmt = conn.prepare(
            "
						INSERT INTO scripts (name, crc, result) 
						VALUES (:name, :crc, :result) ON CONFLICT(name) 
         		DO UPDATE SET crc = excluded.crc, result = excluded.result
						",
        )?;
        let res_text = if result { 1 } else { 0 };
        stmt.execute(named_params! { ":name": file, ":crc": crc, ":result": res_text })?;

        Ok(())
    }

    // pub fn find_many(&self, files: Vec<ListEntry>) -> eyre::Result<Vec<ListEntry>> {
    //     let names: Vec<String> = files
    //         .iter()
    //         .map(|entry| entry.relative_path.clone())
    //         .collect();

    //     let conn = Connection::open(self.db_name.clone())?;

    //     // Build the query dynamically with the appropriate number of placeholders
    //     let placeholders = names.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
    //     let query = format!(
    //         "SELECT name, crc, result FROM scripts WHERE name IN ({})",
    //         placeholders
    //     );

    //     // Prepare the statement and query the database for the matching records
    //     let mut stmt = conn.prepare(&query)?;
    //     let rows = stmt.query_map(params_from_iter(names.clone()), |row| {
    //         Ok(ScriptDatabaseRecord {
    //             name: row.get::<_, String>(0)?, // Using String for name
    //             crc: row.get::<_, String>(1)?,  // Using String for CRC
    //             result: row.get::<_, bool>(2)?, // Using String for CRC
    //         })
    //     })?;

    //     // Build a HashMap from the database results for easier lookup
    //     let mut db_map: HashMap<String, (String, bool)> = HashMap::new(); // String for CRC
    //     for record in rows {
    //         let record = record?;
    //         db_map.insert(record.name, (record.crc, record.result)); // String for CRC
    //     }

    //     // Now classify each ListEntry as Known, Changed, or Unknown
    //     let mut results = Vec::new();

    //     for mut file in files {
    //         if file.is_directory {
    //             file.status = EntryStatus::Directory;
    //         } else if file.crc.is_none() {
    //             file.status = EntryStatus::Unknown;
    //         } else if let Some((db_crc, db_result)) = db_map.get(&file.relative_path) {
    //             if db_crc == file.crc.as_ref().unwrap() {
    //                 if *db_result {
    //                     file.status = EntryStatus::Finished;
    //                 } else {
    //                     file.status = EntryStatus::FinishedWithError;
    //                 }
    //             } else {
    //                 file.status = EntryStatus::Changed;
    //             }
    //         } else {
    //             file.status = EntryStatus::NeverStarted;
    //         }

    //         results.push(file);
    //     }

    //     Ok(results)
    // }

    #[allow(dead_code)]
    pub fn get_file_status(&self, file_path: &str, crc: &u32) -> eyre::Result<EntryStatus> {
        let conn = Connection::open(self.db_name.clone())?;

        // Prepare the query to fetch the matching record for a single file
        let query = "SELECT name, crc, result FROM scripts WHERE name = ?";

        // Prepare the statement and query the database for the matching record
        let mut stmt = conn.prepare(query)?;
        let mut rows = stmt.query_map([file_path], |row| {
            Ok(ScriptDatabaseRecord {
                crc: row.get::<_, u32>(1)?,     // Using String for CRC
                result: row.get::<_, bool>(2)?, // Using bool for result
            })
        })?;

        // Process the results (assuming one record is returned at most)
        match rows.next() {
            Some(record) => match record {
                Ok(record) => {
                    if record.crc == *crc {
                        Ok(EntryStatus::Finished(record.result))
                    } else {
                        Ok(EntryStatus::Changed)
                    }
                }
                Err(e) => {
                    log::error!("Error while processing record: {}", e);
                    Ok(EntryStatus::Unknown)
                }
            },
            None => Ok(EntryStatus::NeverStarted),
        }
    }

    #[cfg(test)]
    pub fn new_test() -> eyre::Result<Self> {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        let temp_path = std::env::temp_dir().join(format!("squealmate_test_{}.db", id));

        let conn = Connection::open(&temp_path)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS scripts (
							name  TEXT NOT NULL PRIMARY KEY,
							result INTEGER NOT NULL,
							crc	 	INTEGER NOT NULL
					)",
            (),
        )?;
        Ok(ScriptDatabase { db_name: temp_path })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_new_database_creates_table() {
        let db = ScriptDatabase::new_test().unwrap();

        // Verify table exists by querying it
        let conn = Connection::open(&db.db_name).unwrap();
        let mut stmt = conn.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='scripts'").unwrap();
        let exists: bool = stmt.exists([]).unwrap();
        assert!(exists, "scripts table should exist");

        // Cleanup
        std::fs::remove_file(&db.db_name).ok();
    }

    #[tokio::test]
    async fn test_insert_new_record() {
        let db = ScriptDatabase::new_test().unwrap();

        // Insert a record
        let result = db.insert("test_script.sql".to_string(), 12345, true);
        assert!(result.is_ok(), "Insert should succeed");

        // Verify it was inserted
        let conn = Connection::open(&db.db_name).unwrap();
        let mut stmt = conn.prepare("SELECT name, crc, result FROM scripts WHERE name = ?").unwrap();
        let mut rows = stmt.query(["test_script.sql"]).unwrap();

        let row = rows.next().unwrap().unwrap();
        assert_eq!(row.get::<_, String>(0).unwrap(), "test_script.sql");
        assert_eq!(row.get::<_, u32>(1).unwrap(), 12345);
        assert_eq!(row.get::<_, i32>(2).unwrap(), 1); // true = 1

        // Cleanup
        std::fs::remove_file(&db.db_name).ok();
    }

    #[tokio::test]
    async fn test_insert_update_existing_record() {
        let db = ScriptDatabase::new_test().unwrap();

        // Insert initial record
        db.insert("test_script.sql".to_string(), 12345, true).unwrap();

        // Update with different CRC and result
        db.insert("test_script.sql".to_string(), 67890, false).unwrap();

        // Verify it was updated
        let conn = Connection::open(&db.db_name).unwrap();
        let mut stmt = conn.prepare("SELECT crc, result FROM scripts WHERE name = ?").unwrap();
        let mut rows = stmt.query(["test_script.sql"]).unwrap();

        let row = rows.next().unwrap().unwrap();
        assert_eq!(row.get::<_, u32>(0).unwrap(), 67890);
        assert_eq!(row.get::<_, i32>(1).unwrap(), 0); // false = 0

        // Cleanup
        std::fs::remove_file(&db.db_name).ok();
    }

    #[tokio::test]
    async fn test_get_file_status_never_started() {
        let db = ScriptDatabase::new_test().unwrap();

        // Query for non-existent file
        let status = db.get_file_status("nonexistent.sql", &12345).unwrap();
        assert!(matches!(status, EntryStatus::NeverStarted));

        // Cleanup
        std::fs::remove_file(&db.db_name).ok();
    }

    #[tokio::test]
    async fn test_get_file_status_finished_success() {
        let db = ScriptDatabase::new_test().unwrap();

        // Insert a successful script
        db.insert("success.sql".to_string(), 12345, true).unwrap();

        // Query with matching CRC
        let status = db.get_file_status("success.sql", &12345).unwrap();
        assert!(matches!(status, EntryStatus::Finished(true)));

        // Cleanup
        std::fs::remove_file(&db.db_name).ok();
    }

    #[tokio::test]
    async fn test_get_file_status_finished_error() {
        let db = ScriptDatabase::new_test().unwrap();

        // Insert a failed script
        db.insert("failed.sql".to_string(), 12345, false).unwrap();

        // Query with matching CRC
        let status = db.get_file_status("failed.sql", &12345).unwrap();
        assert!(matches!(status, EntryStatus::Finished(false)));

        // Cleanup
        std::fs::remove_file(&db.db_name).ok();
    }

    #[tokio::test]
    async fn test_get_file_status_changed() {
        let db = ScriptDatabase::new_test().unwrap();

        // Insert with original CRC
        db.insert("changed.sql".to_string(), 12345, true).unwrap();

        // Query with different CRC
        let status = db.get_file_status("changed.sql", &67890).unwrap();
        assert!(matches!(status, EntryStatus::Changed));

        // Cleanup
        std::fs::remove_file(&db.db_name).ok();
    }

    #[tokio::test]
    async fn test_multiple_scripts() {
        let db = ScriptDatabase::new_test().unwrap();

        // Insert multiple scripts
        db.insert("script1.sql".to_string(), 111, true).unwrap();
        db.insert("script2.sql".to_string(), 222, false).unwrap();
        db.insert("script3.sql".to_string(), 333, true).unwrap();

        // Verify all are stored correctly
        let status1 = db.get_file_status("script1.sql", &111).unwrap();
        let status2 = db.get_file_status("script2.sql", &222).unwrap();
        let status3 = db.get_file_status("script3.sql", &333).unwrap();

        assert!(matches!(status1, EntryStatus::Finished(true)));
        assert!(matches!(status2, EntryStatus::Finished(false)));
        assert!(matches!(status3, EntryStatus::Finished(true)));

        // Cleanup
        std::fs::remove_file(&db.db_name).ok();
    }
}
