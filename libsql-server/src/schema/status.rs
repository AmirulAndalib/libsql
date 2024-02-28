use serde::{Deserialize, Serialize};

use super::Error;

#[derive(Debug, Serialize, Deserialize)]
pub enum MigrationTaskStatus {
    Enqueued,
    DryRunSuccess,
    DryRunFailure { error: String },
    Run,
    Success,
    Failure { error: String },
}

impl MigrationTaskStatus {
    pub fn encode_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn decode_json(s: &str) -> Result<Self, Error> {
        serde_json::from_str(s).map_err(|e| Error::CorruptedJobStatus(e))
    }
}
