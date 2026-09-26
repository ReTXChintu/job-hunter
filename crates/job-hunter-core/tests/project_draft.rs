//! "Add project with Claude": the candidate describes a project in their own
//! words and gets back a structured, unsaved draft. Runs on the mock Claude
//! runner, like the rest of the suite.

use std::sync::Arc;

use job_hunter_core::domain::*;
use job_hunter_core::error::CoreError;
use job_hunter_core::util::now;
use job_hunter_core::AppContext;

async fn ctx_with_two_employers() -> (Arc<AppContext>, String, String) {
    let dir = tempfile::tempdir().unwrap();
    let ctx = AppContext::init_mock(dir.path().join("data"))
        .await
        .unwrap();
    std::mem::forget(dir);
    let mut ids = vec![];
    for (company, role) in [
        ("Bhavnika Pvt. Ltd.", "MERN Stack Developer"),
        ("iBoon Technologies", "Technical Team Lead"),
    ] {
        let e = ctx
            .save_experience(Experience {
                id: String::new(),
                user_id: String::new(),
                company: company.into(),
                role: role.into(),
                location: String::new(),
                start_date: "2022-01".into(),
                end_date: None,
                description: String::new(),
                technologies: vec![],
                achievements: vec![],
                projects: vec![],
                created_at: now(),
                updated_at: now(),
            })
            .unwrap();
        ids.push(e.id);
    }
    (ctx, ids[0].clone(), ids[1].clone())
}

#[tokio::test]
async fn a_description_becomes_an_unsaved_project_linked_to_the_named_employer() {
    let (ctx, bhavnika, _) = ctx_with_two_employers().await;
    let draft = ctx
        .draft_project(
            "At Bhavnika I built a school management system on the MERN stack.",
            None,
        )
        .await
        .unwrap();
    assert_eq!(draft.name, "School Management System");
    assert_eq!(draft.experience_id.as_deref(), Some(bhavnika.as_str()));
    assert_eq!(
        draft.technologies,
        ["MongoDB", "Express.js", "React.js", "Node.js"]
    );
    assert!(draft.id.is_empty(), "a draft has no id until it's saved");
    assert!(
        ctx.projects().unwrap().is_empty(),
        "drafting must not save anything"
    );

    let saved = ctx.save_project(draft).unwrap();
    assert!(!saved.id.is_empty());
    assert_eq!(ctx.projects().unwrap().len(), 1);
}

#[tokio::test]
async fn the_employer_the_candidate_picks_wins_over_the_one_claude_names() {
    let (ctx, _, iboon) = ctx_with_two_employers().await;
    let draft = ctx
        .draft_project(
            "A school management system on the MERN stack.",
            Some(&iboon),
        )
        .await
        .unwrap();
    assert_eq!(draft.experience_id.as_deref(), Some(iboon.as_str()));
}

#[tokio::test]
async fn an_empty_description_is_refused_before_calling_claude() {
    let (ctx, _, _) = ctx_with_two_employers().await;
    let err = ctx.draft_project("  app  ", None).await.unwrap_err();
    assert!(matches!(err, CoreError::Validation(_)));
}
