use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::Entity;
use crate::util::{new_id, normalize_text, now};

/// Reusable application-question answers. Only the user (or the profile) can
/// create these; Claude is never allowed to invent one.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AnswerRecord {
    #[serde(default = "new_id")]
    pub id: String,
    #[serde(default)]
    pub user_id: String,
    pub question: String,
    #[serde(default)]
    pub normalized_question: String,
    pub answer: String,
    /// e.g. "work-authorization", "salary", "notice-period", "custom"
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub times_used: u32,
    #[serde(default = "now")]
    pub created_at: DateTime<Utc>,
    #[serde(default = "now")]
    pub updated_at: DateTime<Utc>,
}

impl AnswerRecord {
    pub fn new(user_id: &str, question: &str, answer: &str, category: &str) -> Self {
        let ts = now();
        Self {
            id: new_id(),
            user_id: user_id.into(),
            question: question.trim().into(),
            normalized_question: normalize_text(question),
            answer: answer.trim().into(),
            category: category.into(),
            times_used: 0,
            created_at: ts,
            updated_at: ts,
        }
    }

    pub fn normalize(&mut self) {
        self.normalized_question = normalize_text(&self.question);
    }
}

impl Entity for AnswerRecord {
    const COLLECTION: &'static str = "application_answers";
    fn id(&self) -> &str {
        &self.id
    }
    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

/// Find the best stored answer for a question using normalised similarity.
pub fn find_answer<'a>(records: &'a [AnswerRecord], question: &str) -> Option<&'a AnswerRecord> {
    let q = normalize_text(question);
    if q.is_empty() {
        return None;
    }
    let mut best: Option<(&AnswerRecord, f64)> = None;
    for r in records {
        let score = strsim::normalized_levenshtein(&q, &r.normalized_question);
        if score >= 0.85 && best.map(|(_, s)| score > s).unwrap_or(true) {
            best = Some((r, score));
        }
    }
    best.map(|(r, _)| r)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_near_duplicate_questions() {
        let records = vec![AnswerRecord::new(
            "u",
            "Are you authorized to work in India?",
            "Yes",
            "work-authorization",
        )];
        assert!(find_answer(&records, "Are you authorised to work in India?").is_some());
        assert!(find_answer(&records, "What is your expected salary?").is_none());
    }
}
