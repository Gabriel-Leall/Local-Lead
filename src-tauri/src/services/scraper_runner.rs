use crate::database::repositories::{create_job, finish_job, upsert_scraper_place};
use crate::database::DbState;
use crate::error::AppError;
use crate::providers::maps_scraper::{adapt_record, build_args, parse_records, zoom_for_radius};
use crate::services::geocode::geocode_city;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;

pub struct RunningScraper {
    child: tokio::process::Child,
    results_path: PathBuf,
    workdir: PathBuf,
    started_at: Instant,
}

/// Processos scraper ativos por job. O comando `start` retorna na hora e o
/// frontend consulta `poll` a cada poucos segundos — sem travar a UI.
pub struct ScraperJobs(pub Mutex<HashMap<i64, RunningScraper>>);

impl Default for ScraperJobs {
    fn default() -> Self {
        Self(Mutex::new(HashMap::new()))
    }
}

fn lock_db(db: &DbState) -> Result<std::sync::MutexGuard<'_, rusqlite::Connection>, AppError> {
    db.0.lock().map_err(|e| AppError::Database(e.to_string()))
}

#[derive(Debug, Clone, Serialize)]
pub struct PollResult {
    pub running: bool,
    pub elapsed_secs: u64,
    pub result_kb: u64,
    pub imported: i64,
    pub merged: i64,
    pub skipped: i64,
}

/// Inicia o scraper em segundo plano e retorna o job na hora. Não precisa de API key.
pub async fn start_scraper_search(
    db: &DbState,
    jobs: &ScraperJobs,
    query: String,
    city: String,
    radius_meters: f64,
    with_email: bool,
) -> Result<i64, AppError> {
    if query.trim().is_empty() || city.trim().is_empty() {
        return Err(AppError::InvalidRequest("informe nicho e cidade".into()));
    }
    if !(500.0..=100_000.0).contains(&radius_meters) {
        return Err(AppError::InvalidRequest("raio entre 0,5 e 100 km".into()));
    }

    let (lat, lng) = geocode_city(&city).await?;

    let job_id = {
        let conn = lock_db(db)?;
        let id = create_job(&conn, &format!("{query} (scraper)"), &city, Some(radius_meters))?;
        conn.execute("UPDATE search_jobs SET strategy='scraper' WHERE id=?1", rusqlite::params![id])?;
        id
    };

    let workdir = std::env::temp_dir().join(format!("local-lead-{job_id}"));
    std::fs::create_dir_all(&workdir)
        .map_err(|e| AppError::Database(format!("pasta temporária: {e}")))?;
    let input_path = workdir.join("queries.txt");
    let results_path = workdir.join("results.json");
    let log_path = workdir.join("scraper.log");
    std::fs::write(&input_path, format!("{query} em {city}\n"))
        .map_err(|e| AppError::Database(format!("escrever consultas: {e}")))?;

    let binary = match crate::providers::maps_scraper::resolve_binary() {
        Ok(b) => b,
        Err(e) => {
            let conn = lock_db(db)?;
            finish_job(&conn, job_id, "failed", 0, 0, Some(&e))?;
            let _ = std::fs::remove_dir_all(&workdir);
            return Err(AppError::InvalidRequest(e));
        }
    };

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

    let log_file = std::fs::File::create(&log_path)
        .map_err(|e| AppError::Database(format!("arquivo de log: {e}")))?;
    let log_file2 = log_file
        .try_clone()
        .map_err(|e| AppError::Database(format!("arquivo de log: {e}")))?;

    let child = tokio::process::Command::new(&binary)
        .args(&args)
        .stdout(std::process::Stdio::from(log_file))
        .stderr(std::process::Stdio::from(log_file2))
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| {
            let msg = if e.kind() == std::io::ErrorKind::NotFound {
                "programa não encontrado — instale ou use Importar JSON".to_string()
            } else {
                format!("falha ao iniciar o scraper: {e}")
            };
            AppError::InvalidRequest(msg)
        });

    let child = match child {
        Ok(c) => c,
        Err(e) => {
            let conn = lock_db(db)?;
            finish_job(&conn, job_id, "failed", 0, 0, Some(&e.to_string()))?;
            let _ = std::fs::remove_dir_all(&workdir);
            return Err(e);
        }
    };

    jobs.0
        .lock()
        .map_err(|e| AppError::Database(e.to_string()))?
        .insert(
            job_id,
            RunningScraper {
                child,
                results_path,
                workdir,
                started_at: Instant::now(),
            },
        );
    Ok(job_id)
}

fn file_kb(path: &PathBuf) -> u64 {
    std::fs::metadata(path).map(|m| m.len() / 1024).unwrap_or(0)
}

fn import_results_file(
    db: &DbState,
    job_id: i64,
    results_path: &PathBuf,
) -> Result<(i64, i64, i64), AppError> {
    let text = std::fs::read_to_string(results_path).map_err(|_| {
        AppError::Provider("scraper terminou mas não gerou resultados — tente de novo".into())
    })?;
    if text.trim().is_empty() {
        return Err(AppError::Provider(
            "scraper não encontrou nada — tente um nicho mais amplo ou raio maior".into(),
        ));
    }
    let records = parse_records(&text).map_err(AppError::InvalidRequest)?;
    let mut imported = 0i64;
    let mut merged = 0i64;
    let mut skipped = 0i64;
    let conn = lock_db(db)?;
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
    Ok((imported, merged, skipped))
}

/// Verifica o processo: se ainda roda, retorna progresso; se terminou,
/// importa os resultados e finaliza o job.
pub async fn poll_scraper_job(
    db: &DbState,
    jobs: &ScraperJobs,
    job_id: i64,
) -> Result<PollResult, AppError> {
    enum Probe {
        Kill(RunningScraper),
        FailExit(RunningScraper, std::process::ExitStatus),
        Import(RunningScraper),
        WaitErr(String),
    }

    let probe = {
        let mut map = jobs
            .0
            .lock()
            .map_err(|e| AppError::Database(e.to_string()))?;
        let Some(entry) = map.get_mut(&job_id) else {
            return Err(AppError::InvalidRequest("job não está mais rodando".into()));
        };
        match entry.child.try_wait() {
            Ok(None) => {
                let elapsed = entry.started_at.elapsed().as_secs();
                let kb = file_kb(&entry.results_path);
                if elapsed > 1200 {
                    Probe::Kill(map.remove(&job_id).unwrap())
                } else {
                    return Ok(PollResult {
                        running: true,
                        elapsed_secs: elapsed,
                        result_kb: kb,
                        imported: 0,
                        merged: 0,
                        skipped: 0,
                    });
                }
            }
            Ok(Some(status)) if !status.success() => {
                Probe::FailExit(map.remove(&job_id).unwrap(), status)
            }
            Ok(Some(_)) => Probe::Import(map.remove(&job_id).unwrap()),
            Err(e) => {
                map.remove(&job_id);
                Probe::WaitErr(e.to_string())
            }
        }
    };

    let running = match probe {
        Probe::Kill(mut entry) => {
            let _ = entry.child.kill().await;
            let conn = lock_db(db)?;
            finish_job(&conn, job_id, "failed", 0, 0, Some("tempo limite de 20 min"))?;
            let _ = std::fs::remove_dir_all(&entry.workdir);
            return Err(AppError::InvalidRequest(
                "scraper demorou mais de 20 min — tente um raio menor".into(),
            ));
        }
        Probe::FailExit(entry, status) => {
            let log =
                std::fs::read_to_string(entry.workdir.join("scraper.log")).unwrap_or_default();
            let tail: String = log
                .lines()
                .rev()
                .take(3)
                .collect::<Vec<_>>()
                .join(" | ")
                .chars()
                .take(300)
                .collect();
            let msg = format!("scraper saiu com erro ({}): {}", status, tail.trim());
            let conn = lock_db(db)?;
            finish_job(&conn, job_id, "failed", 0, 0, Some(&msg))?;
            let _ = std::fs::remove_dir_all(&entry.workdir);
            return Err(AppError::Provider(msg));
        }
        Probe::Import(entry) => entry,
        Probe::WaitErr(e) => {
            let conn = lock_db(db)?;
            finish_job(&conn, job_id, "failed", 0, 0, Some(&e))?;
            return Err(AppError::Provider(format!("falha ao monitorar scraper: {e}")));
        }
    };

    let (imported, merged, skipped) = match import_results_file(db, job_id, &running.results_path) {
        Ok(v) => v,
        Err(e) => {
            let conn = lock_db(db)?;
            finish_job(&conn, job_id, "failed", 0, 0, Some(&e.to_string()))?;
            let _ = std::fs::remove_dir_all(&running.workdir);
            return Err(e);
        }
    };
    {
        let conn = lock_db(db)?;
        if imported + merged == 0 {
            finish_job(&conn, job_id, "completed", 0, 0, None)?;
            let _ = std::fs::remove_dir_all(&running.workdir);
            return Err(AppError::Provider(
                "scraper não encontrou nada — tente um nicho mais amplo ou raio maior".into(),
            ));
        }
        finish_job(&conn, job_id, "completed", imported + merged, imported, None)?;
        crate::services::lead_scoring::rescore_with_saved_config(&conn);
    }
    let elapsed = running.started_at.elapsed().as_secs();
    let _ = std::fs::remove_dir_all(&running.workdir);
    Ok(PollResult {
        running: false,
        elapsed_secs: elapsed,
        result_kb: 0,
        imported,
        merged,
        skipped,
    })
}

pub async fn cancel_scraper_job(
    db: &DbState,
    jobs: &ScraperJobs,
    job_id: i64,
) -> Result<(), AppError> {
    let entry = {
        jobs.0
            .lock()
            .map_err(|e| AppError::Database(e.to_string()))?
            .remove(&job_id)
    };
    if let Some(mut entry) = entry {
        let _ = entry.child.kill().await;
        let _ = std::fs::remove_dir_all(&entry.workdir);
    }
    let conn = lock_db(db)?;
    conn.execute(
        "UPDATE search_jobs SET status='cancelled', completed_at=?1 WHERE id=?2 AND status IN ('running','pending')",
        rusqlite::params![chrono::Utc::now().to_rfc3339(), job_id],
    )?;
    Ok(())
}
