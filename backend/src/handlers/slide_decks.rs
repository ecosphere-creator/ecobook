use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use crate::{
    auth_extractor::AuthUser,
    dto::{CurriculumItemDto, SlideDeckCatalogItemDto, SlideDeckDto, SlideDeckInput, SlideDeckPublicDetailDto},
    error::{AppError, AppResult},
    models::slide_deck::{Slide, SlideDeck},
    repo,
    state::AppState,
};

const AUTHOR_ROLES: &[&str] = &["OWNER", "MENTOR", "MEMBER"];

// ---- access control, ported from SlideDeckService ----

// NOTE (2026-08-12): the paywall is NOT active yet. All published decks are
// fully accessible to everyone — no payment or event-registration gate. When
// the paywall ships, set PAYWALL_ACTIVE to true: the `payments`/`community`
// peer checks in `can_access` below are kept and still type-checked so the
// gate can be re-enabled without re-adding the logic. Until then, the gate is
// deliberately open so the LXS can be plugged into any estate (e.g. the
// getecosphere homepage) with example content.
const PAYWALL_ACTIVE: bool = false;

async fn is_editor(state: &AppState, deck: &SlideDeck, user_id: &str) -> bool {
    if user_id == deck.owner_id {
        return true;
    }
    state.auth_client.is_platform_owner(user_id).await
}

async fn can_access(state: &AppState, deck: &SlideDeck, user_id: Option<&str>) -> bool {
    if !PAYWALL_ACTIVE {
        return true;
    }
    let Some(user_id) = user_id else { return false };
    if user_id == deck.owner_id {
        return true;
    }
    if state.auth_client.is_platform_owner(user_id).await {
        return true;
    }
    if state
        .payments_client
        .has_completed_slide_payment(user_id, &deck.id_string())
        .await
    {
        return true;
    }
    if let Some(event_id) = &deck.event_id {
        if state.community_client.has_active_registration(event_id, user_id).await {
            return true;
        }
    }
    false
}

// ---- slug helpers, ported from SlideDeckService ----

fn slugify(input: Option<&str>) -> Option<String> {
    let input = input?.trim();
    if input.is_empty() {
        return None;
    }
    let lower = input.to_lowercase();
    let normalized: String = lower
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == ' ' || c == '-' || c == '_' { c } else { ' ' })
        .collect();
    let slug = normalized
        .split(|c: char| c.is_whitespace() || c == '_')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if slug.is_empty() {
        None
    } else {
        Some(slug)
    }
}

async fn ensure_unique_slug(state: &AppState, base: &str, self_id: Option<&str>) -> String {
    let mut slug = base.to_string();
    let mut i = 2;
    loop {
        match repo::slide_decks::find_by_slug(state, &slug).await.ok().flatten() {
            None => return slug,
            Some(existing) if self_id == Some(existing.id_string().as_str()) => return slug,
            Some(_) => {
                slug = format!("{base}-{i}");
                i += 1;
            }
        }
    }
}

/// While draft/editable, keep the URL slug available as soon as a title
/// exists. Custom incoming slug still wins; otherwise derive from title.
async fn resolve_editable_slug(state: &AppState, name: Option<&str>, incoming_slug: Option<&str>, self_id: Option<&str>) -> Option<String> {
    let trimmed_incoming = incoming_slug.map(str::trim).filter(|s| !s.is_empty());
    let source = trimmed_incoming.or(name);
    let normalized = slugify(source)?;
    Some(ensure_unique_slug(state, &normalized, self_id).await)
}

async fn ensure_slug_for_published(state: &AppState, deck: &mut SlideDeck) {
    let current = deck.slug.as_deref().map(str::trim).filter(|s| !s.is_empty());
    if let Some(current) = current {
        deck.slug = Some(ensure_unique_slug(state, &slugify(Some(current)).unwrap_or_else(|| "book".to_string()), Some(&deck.id_string())).await);
        return;
    }
    let base = slugify(deck.name.as_deref()).unwrap_or_else(|| "book".to_string());
    deck.slug = Some(ensure_unique_slug(state, &base, Some(&deck.id_string())).await);
}

fn validate_publish_fields(input: &SlideDeckInput) -> AppResult<()> {
    if !input.status.as_deref().is_some_and(|s| s.eq_ignore_ascii_case("published")) {
        return Ok(());
    }
    if input.name.as_deref().unwrap_or("").trim().is_empty() {
        return Err(AppError::BadRequest("Judul wajib diisi sebelum publish".to_string()));
    }
    if input.cover_url.as_deref().unwrap_or("").trim().is_empty() {
        return Err(AppError::BadRequest("Cover wajib diisi sebelum publish".to_string()));
    }
    if input.description.as_deref().unwrap_or("").trim().is_empty() {
        return Err(AppError::BadRequest("Deskripsi singkat wajib diisi sebelum publish".to_string()));
    }
    Ok(())
}

// ---- handlers ----

/// The Java version took the entire `SlideDeck` request body verbatim,
/// including `ownerId` -- a client could set someone else's id (or leave
/// it blank) as the deck's owner. Fixed: always the authenticated caller.
pub async fn create_slide_deck(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<SlideDeckInput>,
) -> AppResult<(StatusCode, Json<SlideDeckDto>)> {
    auth.require_role(AUTHOR_ROLES)?;

    let status = input.status.clone().filter(|s| !s.trim().is_empty()).unwrap_or_else(|| "draft".to_string());
    let slug = resolve_editable_slug(&state, input.name.as_deref(), input.slug.as_deref(), None).await;
    let now = bson::DateTime::now();

    let deck = repo::slide_decks::insert(
        &state,
        SlideDeck {
            id: None,
            name: input.name,
            slug,
            subtitle: input.subtitle,
            cover_url: input.cover_url,
            description: input.description,
            long_summary: input.long_summary,
            learning_objectives: input.learning_objectives,
            requirements: input.requirements,
            target_audience: input.target_audience,
            tags: input.tags,
            level: input.level,
            language: input.language,
            instructor_name: input.instructor_name,
            estimated_duration_minutes: input.estimated_duration_minutes,
            status,
            owner_id: auth.user_id.clone(),
            community_id: input.community_id,
            event_id: input.event_id,
            theme: input.theme,
            transition: input.transition,
            layout_format: input.layout_format,
            price: input.price,
            compare_at_price: input.compare_at_price,
            paywall_start_slide_index: input.paywall_start_slide_index,
            slides: input.slides,
            flow: input.flow,
            gallery_images: input.gallery_images,
            guided_audio_library: input.guided_audio_library,
            created_at: now,
            updated_at: now,
        },
    )
    .await?;

    Ok((StatusCode::CREATED, Json(SlideDeckDto::from(&deck))))
}

pub async fn update_slide_deck(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(input): Json<SlideDeckInput>,
) -> AppResult<Json<SlideDeckDto>> {
    auth.require_role(AUTHOR_ROLES)?;
    let mut existing = repo::slide_decks::find_by_id(&state, &id)
        .await?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("SlideDeck not found")))?;

    if !is_editor(&state, &existing, &auth.user_id).await {
        return Err(AppError::Forbidden("Anda tidak memiliki akses untuk mengubah book ini".to_string()));
    }

    let was_published = existing.status.eq_ignore_ascii_case("published");
    if was_published {
        if let (Some(incoming), Some(current)) = (
            input.slug.as_deref().map(str::trim).filter(|s| !s.is_empty()),
            existing.slug.as_deref().map(str::trim).filter(|s| !s.is_empty()),
        ) {
            if !incoming.eq_ignore_ascii_case(current) {
                return Err(AppError::BadRequest("URL slug tidak bisa diubah setelah publish".to_string()));
            }
        }
    }

    validate_publish_fields(&input)?;

    existing.name = input.name.clone();
    if !was_published {
        existing.slug = resolve_editable_slug(&state, input.name.as_deref(), input.slug.as_deref(), Some(&existing.id_string())).await;
    }
    existing.subtitle = input.subtitle;
    existing.cover_url = input.cover_url;
    existing.description = input.description;
    existing.long_summary = input.long_summary;
    existing.learning_objectives = input.learning_objectives;
    existing.requirements = input.requirements;
    existing.target_audience = input.target_audience;
    existing.tags = input.tags;
    existing.level = input.level;
    existing.language = input.language;
    existing.instructor_name = input.instructor_name;
    existing.estimated_duration_minutes = input.estimated_duration_minutes;

    let incoming_status_published = input.status.as_deref().is_some_and(|s| s.eq_ignore_ascii_case("published"));
    if let Some(status) = input.status.as_deref().filter(|s| !s.trim().is_empty()) {
        existing.status = status.trim().to_lowercase();
    }
    existing.slides = input.slides;
    existing.community_id = input.community_id;
    existing.event_id = input.event_id;
    existing.theme = input.theme;
    existing.transition = input.transition;
    existing.price = input.price;
    existing.compare_at_price = input.compare_at_price;
    existing.paywall_start_slide_index = input.paywall_start_slide_index;
    existing.flow = input.flow;
    existing.gallery_images = input.gallery_images;
    existing.guided_audio_library = input
        .guided_audio_library
        .into_iter()
        .filter(|a| !a.file_id.trim().is_empty())
        .collect();

    if existing.status.eq_ignore_ascii_case("published") && (was_published || incoming_status_published) {
        ensure_slug_for_published(&state, &mut existing).await;
    }
    existing.updated_at = bson::DateTime::now();

    repo::slide_decks::save(&state, &existing).await?;
    Ok(Json(SlideDeckDto::from(&existing)))
}

fn paywall_preview(deck: &SlideDeck) -> SlideDeck {
    let total = deck.slides.len();
    let start = match deck.paywall_start_slide_index {
        Some(s) if s > 0 => s as usize,
        _ => {
            let mut d = deck.clone();
            d.paywall_start_slide_index = None;
            return d;
        }
    };
    if total == 0 || start >= total {
        let mut d = deck.clone();
        d.paywall_start_slide_index = None;
        return d;
    }

    let preview_slides: Vec<Slide> = deck
        .slides
        .iter()
        .enumerate()
        .map(|(i, slide)| {
            if i < start {
                slide.clone()
            } else {
                Slide {
                    id: slide.id.clone(),
                    name: None,
                    elements: Vec::new(),
                    background: slide.background.clone(),
                    level: slide.level,
                    progressive_reveal: None,
                    reveal_order: None,
                    guided_reveal: None,
                    rpp: None,
                }
            }
        })
        .collect();

    let mut d = deck.clone();
    d.slides = preview_slides;
    d.paywall_start_slide_index = Some(start as i32);
    d
}

pub async fn get_slide_deck(
    State(state): State<AppState>,
    auth: Option<AuthUser>,
    Path(id): Path<String>,
) -> AppResult<Json<SlideDeckDto>> {
    let deck = repo::slide_decks::require_by_id_or_slug(&state, &id).await?;
    let caller = auth.as_ref().map(|a| a.user_id.as_str());

    if can_access(&state, &deck, caller).await {
        let mut d = deck.clone();
        d.paywall_start_slide_index = None;
        return Ok(Json(SlideDeckDto::from(&d)));
    }

    Ok(Json(SlideDeckDto::from(&paywall_preview(&deck))))
}

fn extract_slide_title(slide: &Slide, index: usize) -> String {
    for el in &slide.elements {
        if matches!(el.element_type.as_deref(), Some("text") | Some("paragraph")) {
            if let Some(content) = el.content.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                return content.to_string();
            }
        }
    }
    format!("Book {}", index + 1)
}

fn build_curriculum(slides: &[Slide], paywall_start: Option<usize>, has_access: bool) -> Vec<CurriculumItemDto> {
    slides
        .iter()
        .enumerate()
        .map(|(i, slide)| {
            let locked = !has_access && paywall_start.is_some_and(|start| i >= start);
            let title = if locked { format!("Book {}", i + 1) } else { extract_slide_title(slide, i) };
            CurriculumItemDto {
                index: i as i32,
                title,
                level: slide.level.unwrap_or(0),
                locked,
            }
        })
        .collect()
}

fn to_public_detail(deck: &SlideDeck, has_access: bool) -> SlideDeckPublicDetailDto {
    let slide_count = deck.slides.len();
    let paywall_start = match deck.paywall_start_slide_index {
        Some(s) if s > 0 => s as usize,
        _ => 0,
    };
    let effective_start = if slide_count == 0 || paywall_start >= slide_count {
        None
    } else {
        Some(paywall_start)
    };

    SlideDeckPublicDetailDto {
        id: deck.id_string(),
        slug: deck.slug.clone(),
        name: deck.name.clone(),
        subtitle: deck.subtitle.clone(),
        cover_url: deck.cover_url.clone(),
        description: deck.description.clone(),
        long_summary: deck.long_summary.clone(),
        learning_objectives: deck.learning_objectives.clone(),
        requirements: deck.requirements.clone(),
        target_audience: deck.target_audience.clone(),
        tags: deck.tags.clone(),
        level: deck.level.clone(),
        language: deck.language.clone(),
        instructor_name: deck.instructor_name.clone(),
        price: deck.price,
        compare_at_price: deck.compare_at_price,
        estimated_duration_minutes: deck.estimated_duration_minutes,
        slide_count: slide_count as i32,
        paywall_start_slide_index: effective_start.map(|s| s as i32),
        has_access,
        curriculum: build_curriculum(&deck.slides, effective_start, has_access),
        updated_at: deck.updated_at.to_chrono(),
    }
}

pub async fn get_published_catalog(State(state): State<AppState>) -> AppResult<Json<Vec<SlideDeckCatalogItemDto>>> {
    let decks = repo::slide_decks::find_published_for_catalog(&state).await?;
    Ok(Json(decks.iter().map(SlideDeckCatalogItemDto::from).collect()))
}

pub async fn get_slide_deck_public(
    State(state): State<AppState>,
    auth: Option<AuthUser>,
    Path(id_or_slug): Path<String>,
) -> AppResult<Json<SlideDeckPublicDetailDto>> {
    let deck = match repo::slide_decks::find_by_slug(&state, &id_or_slug).await? {
        Some(d) => d,
        None if looks_like_object_id(&id_or_slug) => repo::slide_decks::find_by_id(&state, &id_or_slug)
            .await?
            .ok_or_else(|| AppError::NotFound("Slide deck not found".to_string()))?,
        None => return Err(AppError::NotFound("Slide deck not found".to_string())),
    };
    if !deck.status.eq_ignore_ascii_case("published") {
        return Err(AppError::NotFound("Slide deck not found".to_string()));
    }

    let caller = auth.as_ref().map(|a| a.user_id.as_str());
    let has_access = can_access(&state, &deck, caller).await;
    Ok(Json(to_public_detail(&deck, has_access)))
}

fn looks_like_object_id(value: &str) -> bool {
    value.len() == 24 && value.chars().all(|c| c.is_ascii_hexdigit())
}

/// The Java version had no auth requirement on this endpoint at all
/// (SecurityConfig blanket-permitted every `GET /book/**`), and returned
/// full, un-redacted `SlideDeck` documents -- including unpublished
/// drafts and paywalled slide content -- for any owner id, to anyone.
/// Fixed: caller must be that owner or a platform `OWNER` (the same
/// isEditor check update/delete already used).
pub async fn get_slide_decks_by_owner(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(owner_id): Path<String>,
) -> AppResult<Json<Vec<SlideDeckDto>>> {
    if owner_id != auth.user_id && !state.auth_client.is_platform_owner(&auth.user_id).await {
        return Err(AppError::Forbidden("Anda tidak memiliki akses untuk melihat book ini".to_string()));
    }
    let decks = repo::slide_decks::find_by_owner(&state, &owner_id).await?;
    Ok(Json(decks.iter().map(SlideDeckDto::from).collect()))
}

pub async fn get_slide_decks_by_event(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(event_id): Path<String>,
) -> AppResult<Json<Vec<SlideDeckDto>>> {
    let decks = repo::slide_decks::find_by_event(&state, &event_id).await?;
    let mut out = Vec::new();
    for deck in &decks {
        if can_access(&state, deck, Some(&auth.user_id)).await {
            out.push(SlideDeckDto::from(deck));
        }
    }
    Ok(Json(out))
}

pub async fn delete_slide_deck(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> AppResult<StatusCode> {
    auth.require_role(AUTHOR_ROLES)?;
    let existing = repo::slide_decks::require_by_id_or_slug(&state, &id).await?;
    if !is_editor(&state, &existing, &auth.user_id).await {
        return Err(AppError::Forbidden("Anda tidak memiliki akses untuk menghapus book ini".to_string()));
    }
    repo::slide_decks::delete(&state, &existing.id_string()).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `.ktt` archive export/import (KttArchiveService in the Java version)
/// was deliberately not ported -- see CLAUDE.md. Left as an explicit 501
/// rather than silently dropping the routes.
pub async fn export_slide_deck() -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

pub async fn import_slide_deck() -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}
