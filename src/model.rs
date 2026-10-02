use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Evidence {
    Confirmed,
    Inferred,
}

#[derive(Debug, Serialize)]
pub struct Finding {
    pub subject: String,
    pub kind: String,
    pub summary: String,
    pub facts: Vec<Fact>,
    pub notes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct Fact {
    pub label: String,
    pub value: String,
    pub evidence: Evidence,
}

impl Finding {
    pub fn new(
        subject: impl Into<String>,
        kind: impl Into<String>,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            subject: subject.into(),
            kind: kind.into(),
            summary: summary.into(),
            facts: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn fact(mut self, label: impl Into<String>, value: impl Into<String>) -> Self {
        self.facts.push(Fact {
            label: label.into(),
            value: value.into(),
            evidence: Evidence::Confirmed,
        });
        self
    }

    pub fn inferred_fact(mut self, label: impl Into<String>, value: impl Into<String>) -> Self {
        self.facts.push(Fact {
            label: label.into(),
            value: value.into(),
            evidence: Evidence::Inferred,
        });
        self
    }

    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }
}
