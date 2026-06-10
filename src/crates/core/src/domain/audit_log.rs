use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: String,
    pub event: String,
    pub details: String,
    pub severity: String, // info, warning, error, security
}

pub struct AuditLog {
    entries: Vec<AuditEntry>,
    max_entries: usize,
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new()
    }
}

impl AuditLog {
    pub fn new() -> Self {
        Self { entries: vec![], max_entries: 1000 }
    }

    pub fn record(&mut self, event: &str, details: &str, severity: &str) {
        self.entries.push(AuditEntry {
            timestamp: chrono::Local::now().to_rfc3339(),
            event: event.into(),
            details: details.into(),
            severity: severity.into(),
        });
        if self.entries.len() > self.max_entries {
            self.entries.drain(0..100); // remove oldest 100
        }
    }

    pub fn recent(&self, limit: usize) -> &[AuditEntry] {
        let start = self.entries.len().saturating_sub(limit);
        &self.entries[start..]
    }
}
