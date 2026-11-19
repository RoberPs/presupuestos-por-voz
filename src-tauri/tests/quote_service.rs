//! Preparar, confirmar y generar el PDF del presupuesto.

mod common;

use std::path::{Path, PathBuf};

use app_lib::domain::draft::DraftStatus;
use app_lib::domain::quote_doc::blockers::QuoteWarning;
use app_lib::domain::quote_doc::details::QuoteDetailsInput;
use app_lib::error::{AppError, QuoteBlocker};
use app_lib::services::draft_service::{self as draft, DraftLinePatch};
use app_lib::services::profile_service;
use app_lib::services::quote_service::{self as quote, QuotePreparationView};
use app_lib::state::AppState;
use common::{complete_input, draft_view, example_rates, seed_draft, state_in, EXAMPLE_TEXT};
use tempfile::TempDir;
use time::{Date, Month};

fn today() -> Date {
    Date::from_calendar_date(2026, Month::November, 18).unwrap()
}

struct Fixture {
    _data: TempDir,
    output: TempDir,
    state: AppState,
}

impl Fixture {
    fn path(&self, name: &str) -> PathBuf {
        self.output.path().join(name)
    }
}

/// Perfil completo y borrador de la spec ya revisado.
fn fixture() -> Fixture {
    let data = tempfile::tempdir().unwrap();
    let state = state_in(data.path());
    profile_service::save_profile(&state, complete_input()).unwrap();
    example_rates(&state);
    seed_draft(&state, EXAMPLE_TEXT);
    draft::mark_reviewed(&state).unwrap();
    Fixture {
        _data: data,
        output: tempfile::tempdir().unwrap(),
        state,
    }
}

fn prepare(state: &AppState) -> QuotePreparationView {
    quote::prepare_on(state, today(), false).unwrap()
}

fn save(state: &AppState, change: impl FnOnce(&mut QuoteDetailsInput)) -> QuotePreparationView {
    let mut details = prepare(state).details;
    change(&mut details);
    quote::save_details_on(state, details, today(), false).unwrap()
}

fn with_client(state: &AppState) -> QuotePreparationView {
    save(state, |details| details.client_name = "Ana Ruiz".into())
}

fn generate(state: &AppState, path: &Path) -> Result<quote::SavedPdf, AppError> {
    let token = prepare(state).confirmation_token.expect("debe haber token");
    quote::generate_to_path_on(state, &token, false, path, today())
}

/// Vuelve a dar por bueno el borrador tras cambiar la cantidad de la primera línea.
fn change_first_line(state: &AppState) {
    let line_id = draft_view(state).lines[0].id;
    draft::line_update(
        state,
        line_id,
        DraftLinePatch {
            quantity: Some("9".into()),
            ..DraftLinePatch::default()
        },
    )
    .unwrap();
    draft::mark_reviewed(state).unwrap();
}

#[test]
fn the_document_copies_the_amounts_of_the_draft() {
    let f = fixture();
    with_client(&f.state);
    let document = quote::document_on(&f.state, today(), false).unwrap();

    assert_eq!(document.lines.len(), 2);
    assert_eq!(
        document.lines[0].concept,
        draft_view(&f.state).lines[0].concept
    );
    assert_eq!(document.lines[0].quantity, "3");
    assert_eq!(document.lines[0].unit, "metro");
    assert_eq!(document.lines[0].price, "25,00 €");
    assert_eq!(document.lines[0].amount, "75,00 €");
    assert_eq!(document.lines[1].amount, "60,00 €");
    assert_eq!(document.totals.base, "135,00 €");
    assert_eq!(document.totals.vat_label, "IVA (21 %)");
    assert_eq!(document.totals.vat, "28,35 €");
    assert_eq!(document.totals.total, "163,35 €");
}

#[test]
fn the_document_has_no_internal_information() {
    let f = fixture();
    with_client(&f.state);
    let document = quote::document_on(&f.state, today(), false).unwrap();
    let text = format!("{document:?}");
    assert!(
        !text.contains(EXAMPLE_TEXT),
        "texto transcrito en el documento"
    );
    assert!(
        !text.contains("cambiar"),
        "fragmento del dictado en el documento"
    );
    for internal in ["Tarifa", "pendiente", "revisado", "manual"] {
        assert!(!text.contains(internal), "«{internal}» en el documento");
    }
}

#[test]
fn the_document_shows_the_business_and_the_dates() {
    let f = fixture();
    with_client(&f.state);
    let document = quote::document_on(&f.state, today(), false).unwrap();
    assert_eq!(document.business.trade_name, "Fontanería López");
    assert_eq!(
        document.business.legal_name.as_deref(),
        Some("José López García")
    );
    assert_eq!(document.business.tax_id, "12345678Z");
    assert!(document.business.address.contains("Calle Mayor 1"));
    assert!(document.business.address.contains("28001"));
    assert_eq!(document.client.name, "Ana Ruiz");
    assert_eq!(document.issue_date, "18/11/2026");
    assert_eq!(document.valid_until, "18/12/2026");
}

#[test]
fn the_preparation_summarises_what_will_be_issued() {
    let f = fixture();
    let view = with_client(&f.state);
    assert_eq!(view.details.quote_number, "2026-001");
    assert_eq!(view.details.validity_days, 30);
    assert_eq!(view.issue_date_display, "18/11/2026");
    assert_eq!(view.valid_until_display, "18/12/2026");
    assert_eq!(view.line_count, 2);
    assert_eq!(view.total_display, "163,35 €");
    assert!(view.blockers.is_empty());
    assert!(view.confirmation_token.is_some());
}

#[test]
fn preparing_without_a_draft_is_rejected() {
    let data = tempfile::tempdir().unwrap();
    let state = state_in(data.path());
    assert!(matches!(
        quote::prepare_on(&state, today(), false),
        Err(AppError::InvalidState { .. })
    ));
}

#[test]
fn saving_the_details_does_not_change_the_review_status() {
    let f = fixture();
    with_client(&f.state);
    assert_eq!(draft_view(&f.state).status, DraftStatus::Reviewed);
}

#[test]
fn saved_details_survive_reopening() {
    let f = fixture();
    save(&f.state, |details| {
        details.client_name = "Ana Ruiz".into();
        details.client_phone = Some("611 222 333".into());
        details.validity_days = 15;
    });
    let state = common::reopen(f.state);
    let view = prepare(&state);
    assert_eq!(view.details.client_name, "Ana Ruiz");
    assert_eq!(view.details.client_phone.as_deref(), Some("611 222 333"));
    assert_eq!(view.details.validity_days, 15);
    assert_eq!(view.valid_until_display, "03/12/2026");
}

#[test]
fn invalid_details_are_rejected_and_not_saved() {
    let f = fixture();
    let mut details = prepare(&f.state).details;
    details.client_name = "Ana".into();
    details.validity_days = 0;
    assert!(matches!(
        quote::save_details_on(&f.state, details, today(), false),
        Err(AppError::Validation { .. })
    ));
    assert_eq!(prepare(&f.state).details.client_name, "");
}

#[test]
fn generating_writes_a_pdf() {
    let f = fixture();
    with_client(&f.state);
    let path = f.path("presupuesto.pdf");
    let saved = generate(&f.state, &path).unwrap();
    assert_eq!(saved.page_count, 1);
    assert_eq!(saved.full_path, path);
    assert_eq!(saved.file_name, "presupuesto.pdf");
    assert!(std::fs::read(&path).unwrap().starts_with(b"%PDF"));
}

#[test]
fn generating_does_not_change_the_draft() {
    let f = fixture();
    with_client(&f.state);
    let before = draft_view(&f.state);
    generate(&f.state, &f.path("a.pdf")).unwrap();
    assert_eq!(draft_view(&f.state), before);
}

#[test]
fn preparing_does_not_spend_the_number() {
    let f = fixture();
    with_client(&f.state);
    prepare(&f.state);
    assert_eq!(prepare(&f.state).details.quote_number, "2026-001");
}

#[test]
fn the_number_is_spent_when_the_pdf_is_saved() {
    let f = fixture();
    with_client(&f.state);
    generate(&f.state, &f.path("a.pdf")).unwrap();

    draft::discard(&f.state).unwrap();
    seed_draft(&f.state, EXAMPLE_TEXT);
    draft::mark_reviewed(&f.state).unwrap();

    assert_eq!(prepare(&f.state).details.quote_number, "2026-002");
}

#[test]
fn a_second_pdf_without_changes_keeps_the_number() {
    let f = fixture();
    with_client(&f.state);
    generate(&f.state, &f.path("a.pdf")).unwrap();
    assert_eq!(prepare(&f.state).details.quote_number, "2026-001");
    generate(&f.state, &f.path("b.pdf")).unwrap();
    assert_eq!(prepare(&f.state).details.quote_number, "2026-001");
}

#[test]
fn changing_only_the_client_or_the_validity_keeps_the_number() {
    let f = fixture();
    with_client(&f.state);
    generate(&f.state, &f.path("a.pdf")).unwrap();

    let view = save(&f.state, |details| {
        details.client_phone = Some("611 222 333".into());
        details.validity_days = 60;
    });
    assert_eq!(view.details.quote_number, "2026-001");
}

#[test]
fn changing_a_line_proposes_the_next_number() {
    let f = fixture();
    with_client(&f.state);
    generate(&f.state, &f.path("a.pdf")).unwrap();

    change_first_line(&f.state);

    assert_eq!(prepare(&f.state).details.quote_number, "2026-002");
}

#[test]
fn changing_the_vat_proposes_the_next_number() {
    let f = fixture();
    with_client(&f.state);
    generate(&f.state, &f.path("a.pdf")).unwrap();

    let mut profile = complete_input();
    profile.vat_rate_percent = "10".into();
    profile_service::save_profile(&f.state, profile).unwrap();
    draft::apply_current_vat(&f.state).unwrap();
    draft::mark_reviewed(&f.state).unwrap();

    assert_eq!(prepare(&f.state).details.quote_number, "2026-002");
}

#[test]
fn a_handwritten_number_moves_the_series_forward() {
    let f = fixture();
    save(&f.state, |details| {
        details.client_name = "Ana Ruiz".into();
        details.quote_number = "2026-050".into();
    });
    generate(&f.state, &f.path("a.pdf")).unwrap();

    draft::discard(&f.state).unwrap();
    seed_draft(&f.state, EXAMPLE_TEXT);
    draft::mark_reviewed(&f.state).unwrap();

    assert_eq!(prepare(&f.state).details.quote_number, "2026-051");
}

#[test]
fn a_number_in_another_format_does_not_move_the_series() {
    let f = fixture();
    save(&f.state, |details| {
        details.client_name = "Ana Ruiz".into();
        details.quote_number = "OBRA-7".into();
    });
    generate(&f.state, &f.path("a.pdf")).unwrap();

    draft::discard(&f.state).unwrap();
    seed_draft(&f.state, EXAMPLE_TEXT);
    draft::mark_reviewed(&f.state).unwrap();

    assert_eq!(prepare(&f.state).details.quote_number, "2026-001");
}

#[test]
fn writing_the_proposed_number_keeps_it_automatic() {
    let f = fixture();
    let view = save(&f.state, |details| {
        details.client_name = "Ana Ruiz".into();
        details.quote_number = "2026-001".into();
    });
    assert_eq!(view.details.quote_number, "2026-001");

    let conn = f.state.conn().unwrap();
    let details = app_lib::storage::quote_repo::get_details(&conn)
        .unwrap()
        .unwrap();
    assert_eq!(details.quote_number, None);
}

#[test]
fn an_expired_token_is_rejected() {
    let f = fixture();
    let token = with_client(&f.state).confirmation_token.unwrap();

    save(&f.state, |details| {
        details.client_name = "Otra Persona".into()
    });

    let path = f.path("a.pdf");
    assert!(matches!(
        quote::generate_to_path_on(&f.state, &token, false, &path, today()),
        Err(AppError::QuoteChanged { .. })
    ));
    assert!(!path.exists());
}

#[test]
fn a_token_expires_when_the_date_changes() {
    let f = fixture();
    let token = with_client(&f.state).confirmation_token.unwrap();
    let tomorrow = today().next_day().unwrap();

    let path = f.path("a.pdf");
    assert!(matches!(
        quote::generate_to_path_on(&f.state, &token, false, &path, tomorrow),
        Err(AppError::QuoteChanged { .. })
    ));
    assert!(!path.exists());
}

#[test]
fn simultaneous_generations_are_rejected() {
    let f = fixture();
    let token = with_client(&f.state).confirmation_token.unwrap();
    let _active = f.state.quote.try_begin().unwrap();

    let path = f.path("a.pdf");
    assert!(matches!(
        quote::generate_to_path_on(&f.state, &token, false, &path, today()),
        Err(AppError::InvalidState { .. })
    ));
    assert!(!path.exists());
}

#[test]
fn a_finished_generation_allows_the_next_one() {
    let f = fixture();
    with_client(&f.state);
    generate(&f.state, &f.path("a.pdf")).unwrap();
    generate(&f.state, &f.path("b.pdf")).unwrap();
}

#[test]
fn without_a_client_name_there_is_no_token_and_no_file() {
    let f = fixture();
    let view = prepare(&f.state);
    assert!(view.confirmation_token.is_none());
    assert!(view.blockers.contains(&QuoteBlocker::ClientNameRequired));

    let path = f.path("a.pdf");
    match quote::generate_to_path_on(&f.state, "token", false, &path, today()) {
        Err(AppError::QuoteBlocked { blockers, .. }) => {
            assert!(blockers.contains(&QuoteBlocker::ClientNameRequired))
        }
        other => panic!("resultado inesperado: {other:?}"),
    }
    assert!(!path.exists());
}

// --- Historia 3: elegir dónde se guarda el PDF ---

#[test]
fn the_proposed_name_has_the_number_and_the_date() {
    let f = fixture();
    with_client(&f.state);
    assert_eq!(
        quote::proposed_file_name_on(&f.state, today(), false).unwrap(),
        "Presupuesto 2026-001 2026-11-18.pdf"
    );
}

#[test]
fn characters_not_allowed_in_a_file_name_become_hyphens() {
    let f = fixture();
    save(&f.state, |details| {
        details.client_name = "Ana Ruiz".into();
        details.quote_number = "OBRA/7".into();
    });
    assert_eq!(
        quote::proposed_file_name_on(&f.state, today(), false).unwrap(),
        "Presupuesto OBRA-7 2026-11-18.pdf"
    );

    save(&f.state, |details| {
        details.quote_number = r#"a\b:c*d?e"f<g>h|i"#.into()
    });
    assert_eq!(
        quote::proposed_file_name_on(&f.state, today(), false).unwrap(),
        "Presupuesto a-b-c-d-e-f-g-h-i 2026-11-18.pdf"
    );
}

#[test]
fn the_pdf_extension_is_added_only_when_missing() {
    for (chosen, expected) in [
        ("presupuesto", "presupuesto.pdf"),
        ("presupuesto.pdf", "presupuesto.pdf"),
        ("presupuesto.PDF", "presupuesto.PDF"),
        ("obra.final", "obra.final.pdf"),
    ] {
        assert_eq!(
            quote::with_pdf_extension(Path::new(chosen)),
            PathBuf::from(expected),
            "{chosen}"
        );
    }
}

#[test]
fn a_name_without_extension_is_saved_as_pdf() {
    let f = fixture();
    with_client(&f.state);
    let saved = generate(&f.state, &f.path("presupuesto")).unwrap();
    assert_eq!(saved.file_name, "presupuesto.pdf");
    assert_eq!(saved.full_path, f.path("presupuesto.pdf"));
    assert!(f.path("presupuesto.pdf").exists());
    assert!(!f.path("presupuesto").exists());
}

#[test]
fn a_failed_save_does_not_spend_the_number_or_change_the_draft() {
    let f = fixture();
    with_client(&f.state);
    let before = draft_view(&f.state);
    let path = f.output.path().join("no-existe").join("a.pdf");

    assert!(matches!(
        generate(&f.state, &path),
        Err(AppError::PdfSave { .. })
    ));

    assert_eq!(draft_view(&f.state), before);
    let view = prepare(&f.state);
    assert_eq!(view.details.quote_number, "2026-001");
    let conn = f.state.conn().unwrap();
    assert_eq!(
        app_lib::storage::quote_repo::last_number(&conn, 2026).unwrap(),
        0
    );
    let details = app_lib::storage::quote_repo::get_details(&conn)
        .unwrap()
        .unwrap();
    assert_eq!(details.issued_number, None);
}

#[test]
fn the_token_stays_valid_after_a_failed_save() {
    let f = fixture();
    let token = with_client(&f.state).confirmation_token.unwrap();
    let missing = f.output.path().join("no-existe").join("a.pdf");
    assert!(quote::generate_to_path_on(&f.state, &token, false, &missing, today()).is_err());

    let path = f.path("a.pdf");
    quote::generate_to_path_on(&f.state, &token, false, &path, today()).unwrap();
    assert!(path.exists());
}

// --- Historia 2: impedir el PDF mientras falten datos ---

fn assert_blocked(f: &Fixture, expected: impl Fn(&QuoteBlocker) -> bool) {
    let view = prepare(&f.state);
    assert!(view.confirmation_token.is_none(), "no debe haber token");
    assert!(view.blockers.iter().any(&expected), "{:?}", view.blockers);

    let path = f.path("bloqueado.pdf");
    match quote::generate_to_path_on(&f.state, "token", false, &path, today()) {
        Err(AppError::QuoteBlocked { blockers, .. }) => {
            assert!(blockers.iter().any(&expected), "{blockers:?}")
        }
        other => panic!("resultado inesperado: {other:?}"),
    }
    assert!(!path.exists(), "no debe crearse ningún archivo");
    let conn = f.state.conn().unwrap();
    assert_eq!(
        app_lib::storage::quote_repo::last_number(&conn, 2026).unwrap(),
        0,
        "no debe gastarse el número"
    );
}

#[test]
fn a_draft_without_lines_is_blocked() {
    let f = fixture();
    with_client(&f.state);
    for line in draft_view(&f.state).lines {
        draft::line_delete(&f.state, line.id).unwrap();
    }
    assert_blocked(&f, |blocker| *blocker == QuoteBlocker::NoLines);
}

#[test]
fn a_line_without_a_price_is_blocked_and_named() {
    let f = fixture();
    with_client(&f.state);
    let line = draft_view(&f.state).lines[0].clone();
    f.state
        .conn()
        .unwrap()
        .execute(
            "UPDATE draft_lines SET price_cents = NULL, price_source = 'pending', rate_id = NULL,
                    rate_concept = NULL, pending_reason = 'no_rate' WHERE id = ?1",
            [line.id],
        )
        .unwrap();
    assert_blocked(&f, |blocker| {
        matches!(blocker, QuoteBlocker::PendingLines { lines }
            if lines.len() == 1 && lines[0].id == line.id && lines[0].concept == line.concept)
    });
}

#[test]
fn a_line_without_a_quantity_is_blocked() {
    let f = fixture();
    with_client(&f.state);
    f.state
        .conn()
        .unwrap()
        .execute(
            "UPDATE draft_lines SET quantity_centi = NULL, quantity_issue = 'missing'
             WHERE position = 0",
            [],
        )
        .unwrap();
    assert_blocked(&f, |blocker| {
        matches!(blocker, QuoteBlocker::PendingLines { .. })
    });
}

#[test]
fn a_draft_that_was_edited_needs_a_new_review() {
    let f = fixture();
    with_client(&f.state);
    let line_id = draft_view(&f.state).lines[0].id;
    draft::line_update(
        &f.state,
        line_id,
        DraftLinePatch {
            quantity: Some("9".into()),
            ..DraftLinePatch::default()
        },
    )
    .unwrap();
    assert_blocked(&f, |blocker| *blocker == QuoteBlocker::NotReviewed);
}

#[test]
fn an_incomplete_profile_is_blocked() {
    let f = fixture();
    with_client(&f.state);
    let mut profile = complete_input();
    profile.phone = String::new();
    profile_service::save_profile(&f.state, profile).unwrap();
    assert_blocked(&f, |blocker| {
        matches!(blocker, QuoteBlocker::ProfileIncomplete { missing_fields }
            if missing_fields.iter().any(|field| field.field == "phone"))
    });
}

#[test]
fn a_business_without_a_profile_is_blocked() {
    let data = tempfile::tempdir().unwrap();
    let state = state_in(data.path());
    example_rates(&state);
    seed_draft(&state, EXAMPLE_TEXT);
    draft::mark_reviewed(&state).unwrap();
    let view = prepare(&state);
    assert!(view
        .blockers
        .iter()
        .any(|blocker| matches!(blocker, QuoteBlocker::ProfileIncomplete { .. })));
}

#[test]
fn an_emoji_in_a_concept_is_blocked() {
    let f = fixture();
    with_client(&f.state);
    let line_id = draft_view(&f.state).lines[0].id;
    draft::line_update(
        &f.state,
        line_id,
        DraftLinePatch {
            concept: Some("Bajante 😀".into()),
            ..DraftLinePatch::default()
        },
    )
    .unwrap();
    draft::mark_reviewed(&f.state).unwrap();
    assert_blocked(&f, |blocker| {
        matches!(blocker, QuoteBlocker::UnsupportedCharacters { fields }
            if fields[0].characters == ["😀"] && fields[0].label.contains("Bajante"))
    });
}

#[test]
fn an_emoji_in_the_client_is_blocked() {
    let f = fixture();
    save(&f.state, |details| details.client_name = "Ana 😀".into());
    assert_blocked(&f, |blocker| {
        matches!(blocker, QuoteBlocker::UnsupportedCharacters { fields }
            if fields[0].field == "clientName" && fields[0].characters == ["😀"])
    });
}

#[test]
fn an_emoji_in_the_business_profile_is_blocked() {
    let f = fixture();
    with_client(&f.state);
    let mut profile = complete_input();
    profile.trade_name = "Fontanería 🔧".into();
    profile_service::save_profile(&f.state, profile).unwrap();
    assert_blocked(&f, |blocker| {
        matches!(blocker, QuoteBlocker::UnsupportedCharacters { fields }
            if fields[0].field == "tradeName" && fields[0].characters == ["🔧"])
    });
}

/// Logo PNG guardado en el perfil.
fn with_logo(f: &Fixture) -> PathBuf {
    let source = common::write_png_rgba(f.output.path(), "logo.png", 300, 300);
    app_lib::services::logo_service::set_logo(&f.state, &source).unwrap();
    let conn = f.state.conn().unwrap();
    let profile = app_lib::storage::profile_repo::get(&conn).unwrap().unwrap();
    f.state.data_dir.join(profile.logo.unwrap().file)
}

#[test]
fn a_readable_logo_goes_into_the_document_without_warnings() {
    let f = fixture();
    with_logo(&f);
    let view = with_client(&f.state);
    assert!(view.warnings.is_empty(), "{:?}", view.warnings);
    assert!(quote::document_on(&f.state, today(), false)
        .unwrap()
        .logo
        .is_some());
    generate(&f.state, &f.path("a.pdf")).unwrap();
}

#[test]
fn an_unreadable_logo_blocks_unless_accepted() {
    let f = fixture();
    let logo = with_logo(&f);
    with_client(&f.state);
    std::fs::write(&logo, b"archivo danado").unwrap();

    assert_blocked(&f, |blocker| *blocker == QuoteBlocker::LogoUnreadable);

    let view = quote::prepare_on(&f.state, today(), true).unwrap();
    assert!(view.blockers.is_empty(), "{:?}", view.blockers);
    assert_eq!(view.warnings, [QuoteWarning::LogoUnreadable]);

    let path = f.path("sin-logo.pdf");
    let token = view.confirmation_token.unwrap();
    quote::generate_to_path_on(&f.state, &token, true, &path, today()).unwrap();
    assert!(path.exists());
    assert!(quote::document_on(&f.state, today(), true)
        .unwrap()
        .logo
        .is_none());
}

#[test]
fn a_deleted_logo_file_blocks_too() {
    let f = fixture();
    let logo = with_logo(&f);
    with_client(&f.state);
    std::fs::remove_file(&logo).unwrap();
    assert_blocked(&f, |blocker| *blocker == QuoteBlocker::LogoUnreadable);
}

#[test]
fn a_business_without_a_logo_is_warned_and_not_blocked() {
    let f = fixture();
    let view = with_client(&f.state);
    assert_eq!(view.warnings, [QuoteWarning::NoLogo]);
    assert!(view.confirmation_token.is_some());
}

#[test]
fn a_vat_change_is_warned_and_does_not_block() {
    let f = fixture();
    with_client(&f.state);
    let mut profile = complete_input();
    profile.vat_rate_percent = "10".into();
    profile_service::save_profile(&f.state, profile).unwrap();

    let view = prepare(&f.state);
    assert!(view.warnings.contains(&QuoteWarning::VatChanged {
        applied_rate_percent: "21".into(),
        current_rate_percent: "10".into(),
    }));
    assert!(view.blockers.is_empty());
    assert_eq!(
        view.total_display, "163,35 €",
        "se emite con el tipo aplicado"
    );
    generate(&f.state, &f.path("a.pdf")).unwrap();
}

#[test]
fn updating_the_vat_returns_the_draft_to_review() {
    let f = fixture();
    with_client(&f.state);
    let mut profile = complete_input();
    profile.vat_rate_percent = "10".into();
    profile_service::save_profile(&f.state, profile).unwrap();

    draft::apply_current_vat(&f.state).unwrap();

    let view = prepare(&f.state);
    assert!(view.blockers.contains(&QuoteBlocker::NotReviewed));
    assert_eq!(view.total_display, "148,50 €");
}

#[test]
fn unparsed_fragments_are_warned_and_do_not_block() {
    let f = fixture();
    with_client(&f.state);
    f.state
        .conn()
        .unwrap()
        .execute(
            "INSERT INTO draft_unparsed_fragments (draft_id, position, text)
             VALUES (1, 0, 'algo sin entender')",
            [],
        )
        .unwrap();

    let view = prepare(&f.state);
    assert!(view
        .warnings
        .contains(&QuoteWarning::UnparsedFragments { count: 1 }));
    assert!(view.confirmation_token.is_some());

    let document = quote::document_on(&f.state, today(), false).unwrap();
    assert!(!format!("{document:?}").contains("algo sin entender"));
}

#[test]
fn without_a_number_there_is_no_token_and_no_file() {
    let f = fixture();
    let view = save(&f.state, |details| {
        details.client_name = "Ana Ruiz".into();
        details.quote_number = "   ".into();
    });
    assert!(view.confirmation_token.is_none());
    assert!(view.blockers.contains(&QuoteBlocker::QuoteNumberRequired));
    assert_eq!(view.details.quote_number, "");

    let path = f.path("a.pdf");
    assert!(matches!(
        quote::generate_to_path_on(&f.state, "token", false, &path, today()),
        Err(AppError::QuoteBlocked { .. })
    ));
    assert!(!path.exists());
}
