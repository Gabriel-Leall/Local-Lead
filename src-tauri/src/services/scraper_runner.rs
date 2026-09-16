use crate::database::repositories::{create_job, finish_job, upsert_scraper_place};
use crate::database::DbState;
use crate::error::AppError;
use crate::providers::maps_scraper::{adapt_record, build_args, parse_records, zoom_for_radius};
use crate::services::geocode::geocode_city;

fn lock(db: &DbState) -> Result<std::sync::MutexGuard<'_, rusqlite::Connection>, AppError> {
    db.0.lock().map_err(|e| AppError::Database(e.to_string()))
}

pub struct ScraperRunResult {
    pub job_id: i64,
    pub imported: i64,
    pub merged: i64,
    pub skipped: i64,
}

const INSTALL_HINT: &str = "instale o binário (https://github.com/gosom/google-maps-scraper/releases) no PATH ou use Importar JSON";

/// Busca completa via scraper local, sem chave de API:
/// geocodifica a cidade → roda o binário → importa o JSON → deduplica.
pub async fn run_scraper_search(
    db: &DbState,
    query: String,
    city: String,
    radius_meters: f64,
    with_email: bool,
) -> Result<ScraperRunResult, AppError> {
    if query.trim().is_empty() || city.trim().is_empty() {
        return Err(AppError::InvalidRequest("informe nicho e cidade".into()));
    }
    if !(500.0..=100_000.0).contains(&radius_meters) {
        return Err(AppError::InvalidRequest("raio entre 0,5 e 100 km".into()));
    }

    let (lat, lng) = geocode_city(&city).await?;

    let job_id = {
        let conn = lock(db)?;
        let id = create_job(&conn, &format!("{query} (scraper)"), &city, Some(radius_meters))?;
        conn.execute("UPDATE search_jobs SET strategy='scraper' WHERE id=?1", rusqlite::params![id])?;
        id
    };

    let workdir = std::env::temp_dir().join(format!("local-lead-{job_id}"));
    if let Err(e) = std::fs::create_dir_all(&workdir) {
        let conn = lock(db)?;
        finish_job(&conn, job_id, "failed", 0, 0, Some(&e.to_string()))?;
        return Err(AppError::Database(format!("não foi possível criar pasta temporária: {e}")));
    }
    let input_path = workdir.join("queries.txt");
    let results_path = workdir.join("results.json");
    if let Err(e) = std::fs::write(&input_path, format!("{query} em {city}\n")) {
        let conn = lock(db)?;
        finish_job(&conn, job_id, "failed", 0, 0, Some(&e.to_string()))?;
        return Err(AppError::Database(format!("não foi possível escrever consultas: {e}")));
    }

    let geo = format!("{lat},{lng}");
    let args = build_args(
        &geo,
        zoom_for_radius(radius_meters / 1000.0),
        radius_meters,
        &input_path.to_string_lossy(),
        &results_path.to_string_lossy(),
        with_email,
        2,
    );

    let binary = match crate::providers::maps_scraper::resolve_binary() {
        Ok(b) => b,
        Err(e) => {
            let conn = lock(db)?;
            finish_job(&conn, job_id, "failed", 0, 0, Some(&e))?;
            let _ = std::fs::remove_dir_all(&workdir);
            return Err(AppError::InvalidRequest(e));
        }
    };

    let output = tokio::time::timeout(
        std::time::Duration::from_secs(900),
        tokio::process::Command::new(&binary)
            .args(&args)
            .kill_on_drop(true)
            .output(),
    )
    .await
    .map_err(|_| {
        AppError::InvalidRequest("scraper demorou mais de 15 min — tente um raio menor".into())
    })
    .and_then(|r| {
        r.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                AppError::InvalidRequest(format!("programa não encontrado — {INSTALL_HINT}"))
            } else {
                AppError::InvalidRequest(format!("falha ao executar o scraper: {e}"))
            }
        })
    });

    let output = match output {
        Ok(o) => o,
        Err(e) => {
            let conn = lock(db)?;
            finish_job(&conn, job_id, "failed", 0, 0, Some(&e.to_string()))?;
            let _ = std::fs::remove_dir_all(&workdir);
            return Err(e);
        }
    };

    if !output.status.success() {
        let stderr: String = String::from_utf8_lossy(&output.stderr).chars().take(300).collect();
        let msg = format!("scraper saiu com erro ({}): {}", output.status, stderr.trim());
        {
            let conn = lock(db)?;
            finish_job(&conn, job_id, "failed", 0, 0, Some(&msg))?;
        }
        let _ = std::fs::remove_dir_all(&workdir);
        return Err(AppError::Provider(msg));
    }

    let text = std::fs::read_to_string(&results_path).map_err(|_| {
        AppError::Provider("scraper terminou mas não gerou resultados — tente de novo".into())
    })?;
    if text.len() > 20_000_000 {
        let conn = lock(db)?;
        finish_job(&conn, job_id, "failed", 0, 0, Some("results too large"))?;
        let _ = std::fs::remove_dir_all(&workdir);
        return Err(AppError::InvalidRequest("resultado grande demais (>20MB)".into()));
    }
    let records = parse_records(&text).map_err(AppError::InvalidRequest)?;

    let mut imported = 0i64;
    let mut merged = 0i64;
    let mut skipped = 0i64;
    {
        let conn = lock(db)?;
        for (idx, rec) in records.iter().enumerate() {
            let Some(place) = adapt_record(rec, idx) else {
                skipped += 1;
                continue;
            };
            match upsert_scraper_place(&conn, job_id, None, &place) {
                Ok((_, is_new)) => {
                    if is_new {
                        imported += 1;
                    } else {
                        merged += 1;
                    }
                }
                Err(_) => skipped += 1,
            }
            if let Some(email) = rec.primary_email() {
                let _ = conn.execute(
                    "UPDATE leads SET email=COALESCE(email, ?1) WHERE id IN (SELECT lead_id FROM external_places WHERE provider='maps_scraper' AND external_place_id=?2)",
                    rusqlite::params![email, place.external_id],
                );
            }
        }
        finish_job(&conn, job_id, "completed", imported + merged, imported, None)?;
    }
    let _ = std::fs::remove_dir_all(&workdir);
    Ok(ScraperRunResult { job_id, imported, merged, skipped })
}
