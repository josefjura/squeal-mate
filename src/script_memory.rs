use crate::{config::get_script_database, entries::EntryStatus};
use color_eyre::eyre::{self};
use rusqlite::{named_params, Connection};
use std::path::PathBuf;

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
}
