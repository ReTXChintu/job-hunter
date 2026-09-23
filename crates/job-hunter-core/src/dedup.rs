//! Job de-duplication. The same job shows up on several platforms; one real
//! job must lead to at most one application.

use url::Url;

use crate::domain::{Job, JobSource};
use crate::util::normalize_text;

/// Strip tracking parameters and fragments so two links to the same posting
/// compare equal.
pub fn canonical_url(raw: &str) -> String {
    let Ok(mut url) = Url::parse(raw.trim()) else {
        return raw.trim().to_lowercase();
    };
    url.set_fragment(None);
    let keep: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(k, _)| {
            let k = k.to_lowercase();
            !(k.starts_with("utm_")
                || k == "ref"
                || k == "refid"
                || k == "trk"
                || k == "trackingid"
                || k == "src"
                || k == "source"
                || k == "gh_src"
                || k == "lever-source"
                || k == "originalsubdomain"
                || k == "position"
                || k == "pagenum")
        })
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    url.set_query(None);
    if !keep.is_empty() {
        let mut q = url.query_pairs_mut();
        for (k, v) in keep {
            q.append_pair(&k, &v);
        }
    }
    let mut s = url.to_string();
    if let Some(host) = url.host_str() {
        let lower = host.to_lowercase();
        s = s.replacen(host, lower.trim_start_matches("www."), 1);
    }
    s.trim_end_matches('/').to_string()
}

fn normalize_company(company: &str) -> String {
    let n = normalize_text(company);
    let mut words: Vec<&str> = n.split(' ').collect();
    while let Some(last) = words.last() {
        if matches!(
            *last,
            "inc"
                | "ltd"
                | "llc"
                | "pvt"
                | "private"
                | "limited"
                | "corp"
                | "corporation"
                | "technologies"
                | "technology"
                | "co"
                | "plc"
                | "gmbh"
        ) {
            words.pop();
        } else {
            break;
        }
    }
    words.join(" ")
}

fn normalize_title(title: &str) -> String {
    let n = normalize_text(title);
    n.split(' ')
        .filter(|w| {
            !matches!(
                *w,
                "the"
                    | "a"
                    | "an"
                    | "and"
                    | "of"
                    | "for"
                    | "at"
                    | "in"
                    | "with"
                    | "role"
                    | "position"
                    | "job"
                    | "hiring"
                    | "urgent"
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_location(location: &str) -> String {
    let n = normalize_text(location);
    n.split(' ').take(2).collect::<Vec<_>>().join(" ")
}

/// Stable key for exact-duplicate detection.
pub fn dedup_key(company: &str, title: &str, location: &str) -> String {
    format!(
        "{}|{}|{}",
        normalize_company(company),
        normalize_title(title),
        normalize_location(location)
    )
}

/// How similar two jobs are, 0.0..1.0.
pub fn similarity(a: &Job, b: &Job) -> f64 {
    if !a.canonical_url.as_deref().unwrap_or("").is_empty() && a.canonical_url == b.canonical_url {
        return 1.0;
    }
    if let (Some(x), Some(y)) = (&a.source_job_id, &b.source_job_id) {
        if !x.is_empty() && x == y && a.source == b.source {
            return 1.0;
        }
    }
    let company = strsim::jaro_winkler(
        &normalize_company(&a.company),
        &normalize_company(&b.company),
    );
    if company < 0.85 {
        return 0.0;
    }
    let title = strsim::jaro_winkler(&normalize_title(&a.title), &normalize_title(&b.title));
    let location = {
        let la = normalize_location(&a.location);
        let lb = normalize_location(&b.location);
        if la.is_empty() || lb.is_empty() {
            0.8
        } else {
            strsim::jaro_winkler(&la, &lb)
        }
    };
    let description = {
        let da = normalize_text(&a.description);
        let db = normalize_text(&b.description);
        if da.len() < 80 || db.len() < 80 {
            0.7
        } else {
            token_overlap(&da, &db)
        }
    };
    0.35 * company + 0.35 * title + 0.1 * location + 0.2 * description
}

fn token_overlap(a: &str, b: &str) -> f64 {
    use std::collections::HashSet;
    let sa: HashSet<&str> = a.split(' ').filter(|w| w.len() > 3).collect();
    let sb: HashSet<&str> = b.split(' ').filter(|w| w.len() > 3).collect();
    if sa.is_empty() || sb.is_empty() {
        return 0.0;
    }
    let inter = sa.intersection(&sb).count() as f64;
    let union = sa.union(&sb).count() as f64;
    inter / union
}

pub const DUPLICATE_THRESHOLD: f64 = 0.9;

pub fn is_duplicate(a: &Job, b: &Job) -> bool {
    similarity(a, b) >= DUPLICATE_THRESHOLD
}

/// Merge `incoming` into `existing`: keep the richer description and add the
/// new source link.
pub fn merge_into(existing: &mut Job, incoming: &Job) {
    for s in &incoming.sources {
        if !existing
            .sources
            .iter()
            .any(|e| canonical_url(&e.url) == canonical_url(&s.url))
        {
            existing.sources.push(JobSource {
                platform: s.platform.clone(),
                url: s.url.clone(),
                source_job_id: s.source_job_id.clone(),
            });
        }
    }
    if incoming.description.len() > existing.description.len() {
        existing.description = incoming.description.clone();
        existing.details_complete = incoming.details_complete || existing.details_complete;
    }
    if existing.requirements.is_empty() {
        existing.requirements = incoming.requirements.clone();
    }
    if existing.responsibilities.is_empty() {
        existing.responsibilities = incoming.responsibilities.clone();
    }
    for s in &incoming.skills {
        if !existing.skills.iter().any(|e| e.eq_ignore_ascii_case(s)) {
            existing.skills.push(s.clone());
        }
    }
    if existing.salary.is_none() {
        existing.salary = incoming.salary.clone();
    }
    if existing.posted_at.is_none() {
        existing.posted_at = incoming.posted_at.clone();
    }
    if existing.location.is_empty() {
        existing.location = incoming.location.clone();
    }
    existing.touch();
}

/// Prepare a job for storage: compute canonical url and dedup key.
pub fn prepare(job: &mut Job) {
    job.canonical_url = Some(canonical_url(&job.url));
    job.dedup_key = dedup_key(&job.company, &job.title, &job.location);
}

/// Result of de-duplicating a batch of incoming jobs against existing ones.
pub struct DedupOutcome {
    pub new_jobs: Vec<Job>,
    /// (existing job id, merged job) pairs that must be re-saved.
    pub merged: Vec<Job>,
    pub duplicates_removed: usize,
}

pub fn deduplicate(existing: &[Job], incoming: Vec<Job>) -> DedupOutcome {
    let mut pool: Vec<Job> = existing.to_vec();
    let mut new_jobs: Vec<Job> = Vec::new();
    let mut merged_ids: Vec<String> = Vec::new();
    let mut duplicates_removed = 0;

    for mut job in incoming {
        prepare(&mut job);
        let mut matched: Option<usize> = None;
        for (i, candidate) in pool.iter().enumerate() {
            if candidate.dedup_key == job.dedup_key || is_duplicate(candidate, &job) {
                matched = Some(i);
                break;
            }
        }
        match matched {
            Some(i) => {
                duplicates_removed += 1;
                let target = &mut pool[i];
                merge_into(target, &job);
                if existing.iter().any(|e| e.id == target.id) && !merged_ids.contains(&target.id) {
                    merged_ids.push(target.id.clone());
                } else if let Some(n) = new_jobs.iter_mut().find(|n| n.id == target.id) {
                    *n = target.clone();
                }
            }
            None => {
                pool.push(job.clone());
                new_jobs.push(job);
            }
        }
    }
    let merged = pool
        .into_iter()
        .filter(|j| merged_ids.contains(&j.id))
        .collect();
    DedupOutcome {
        new_jobs,
        merged,
        duplicates_removed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(source: &str, url: &str, company: &str, title: &str, location: &str, desc: &str) -> Job {
        let mut j = Job::new("u", source, url, company, title);
        j.location = location.into();
        j.description = desc.into();
        j
    }

    #[test]
    fn canonical_url_strips_tracking() {
        assert_eq!(
            canonical_url(
                "https://www.linkedin.com/jobs/view/123?refId=abc&trackingId=xyz&utm_source=x#top"
            ),
            "https://linkedin.com/jobs/view/123"
        );
        assert_eq!(
            canonical_url("https://boards.greenhouse.io/acme/jobs/42?gh_src=foo"),
            "https://boards.greenhouse.io/acme/jobs/42"
        );
    }

    #[test]
    fn same_job_on_two_platforms_is_deduplicated() {
        let desc = "We are looking for a senior full stack developer with React, Node.js and PostgreSQL experience to join our platform team building customer facing applications at scale.";
        let a = job(
            "LinkedIn",
            "https://linkedin.com/jobs/view/1",
            "ABC Technologies Pvt Ltd",
            "Senior Full Stack Developer",
            "Mumbai, Maharashtra",
            desc,
        );
        let b = job(
            "Naukri",
            "https://naukri.com/job/2",
            "ABC Technologies",
            "Senior Full-Stack Developer",
            "Mumbai",
            desc,
        );
        let out = deduplicate(std::slice::from_ref(&a), vec![b]);
        assert_eq!(out.new_jobs.len(), 0);
        assert_eq!(out.duplicates_removed, 1);
        assert_eq!(out.merged.len(), 1);
        assert_eq!(out.merged[0].sources.len(), 2);
    }

    #[test]
    fn different_jobs_at_same_company_are_kept() {
        let a = job(
            "LinkedIn",
            "https://linkedin.com/jobs/view/1",
            "ABC Technologies",
            "Senior Full Stack Developer",
            "Mumbai",
            "React and Node.js role in the platform team working on web applications and APIs.",
        );
        let b = job(
            "LinkedIn",
            "https://linkedin.com/jobs/view/2",
            "ABC Technologies",
            "DevOps Engineer",
            "Mumbai",
            "Kubernetes, Terraform and AWS infrastructure automation for a growing SaaS product.",
        );
        let out = deduplicate(&[a], vec![b]);
        assert_eq!(out.new_jobs.len(), 1);
        assert_eq!(out.duplicates_removed, 0);
    }

    #[test]
    fn duplicates_within_a_single_batch_collapse() {
        let a = job(
            "LinkedIn",
            "https://linkedin.com/jobs/view/1?trk=a",
            "Acme",
            "Backend Engineer",
            "Remote",
            "",
        );
        let b = job(
            "LinkedIn",
            "https://linkedin.com/jobs/view/1?trk=b",
            "Acme",
            "Backend Engineer",
            "Remote",
            "",
        );
        let out = deduplicate(&[], vec![a, b]);
        assert_eq!(out.new_jobs.len(), 1);
        assert_eq!(out.duplicates_removed, 1);
    }
}
