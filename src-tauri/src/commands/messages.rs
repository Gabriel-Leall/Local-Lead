use crate::database::repositories::{create_message, get_lead, list_messages, set_message_status};
use crate::database::DbState;
use crate::domain::Lead;
use crate::error::AppError;
use crate::services::message_generator::{builtin_templates, extract_city, render_template, LeadContext, MessageTemplate};
use serde::Serialize;
use tauri::State;

fn lock(db: &DbState) -> Result<std::sync::MutexGuard<'_, rusqlite::Connection>, AppError> {
    db.0.lock().map_err(|e| AppError::Database(e.to_string()))
}

fn ctx_from_lead(lead: &Lead) -> LeadContext {
    LeadContext {
        name: lead.canonical_name.clone(),
        category: lead.category.clone(),
        city: extract_city(lead.address.as_deref()),
        website: lead.website.clone(),
        instagram: lead.instagram.clone(),
        rating: lead.rating,
        reviews: lead.review_count,
        phone: lead.phone.clone(),
        email: lead.email.clone(),
    }
}

#[tauri::command]
pub fn list_templates() -> Vec<MessageTemplate> {
    builtin_templates()
}

#[derive(Serialize)]
pub struct PreviewResponse {
    pub subject: String,
    pub message: String,
}

#[tauri::command]
pub fn preview_message(
    db: State<'_, DbState>,
    lead_id: i64,
    template_id: String,
) -> Result<PreviewResponse, AppError> {
    let conn = lock(&db)?;
    let lead = get_lead(&conn, lead_id)?.ok_or_else(|| AppError::InvalidRequest("lead not found".into()))?;
    let t = builtin_templates()
        .into_iter()
        .find(|t| t.id == template_id)
        .ok_or_else(|| AppError::InvalidRequest("unknown template".into()))?;
    let (subject, message) = render_template(&t, &ctx_from_lead(&lead));
    Ok(PreviewResponse { subject, message })
}

#[tauri::command]
pub fn generate_message(
    db: State<'_, DbState>,
    lead_id: i64,
    template_id: String,
    channel: Option<String>,
) -> Result<i64, AppError> {
    let conn = lock(&db)?;
    let lead = get_lead(&conn, lead_id)?.ok_or_else(|| AppError::InvalidRequest("lead not found".into()))?;
    let t = builtin_templates()
        .into_iter()
        .find(|t| t.id == template_id)
        .ok_or_else(|| AppError::InvalidRequest("unknown template".into()))?;
    let (subject, message) = render_template(&t, &ctx_from_lead(&lead));
    let ch = channel.unwrap_or(t.channel.to_string());
    Ok(create_message(&conn, lead_id, &ch, Some(&subject), &message, Some(t.id))?)
}

#[tauri::command]
pub fn get_lead_detail(db: State<'_, DbState>, lead_id: i64) -> Result<Option<Lead>, AppError> {
    let conn = lock(&db)?;
    Ok(get_lead(&conn, lead_id)?)
}

#[tauri::command]
pub fn get_lead_messages(
    db: State<'_, DbState>,
    lead_id: i64,
) -> Result<Vec<crate::database::repositories::OutreachMessage>, AppError> {
    let conn = lock(&db)?;
    Ok(list_messages(&conn, lead_id)?)
}

#[tauri::command]
pub fn set_outreach_status(
    db: State<'_, DbState>,
    id: i64,
    status: String,
) -> Result<(), AppError> {
    let conn = lock(&db)?;
    set_message_status(&conn, id, &status).map_err(|e| AppError::InvalidRequest(e.to_string()))
}
