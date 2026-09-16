use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("erro de banco de dados: {0}")]
    Database(String),
    #[error("erro do provedor: {0}")]
    Provider(String),
    #[error("chave de API inválida ou sem permissão — verifique a chave e se a Places API (New) está ativada com faturamento")]
    InvalidApiKey,
    #[error("limite de requisições excedido — aguarde e tente de novo")]
    RateLimit,
    #[error("erro de rede: {0}")]
    Network(String),
    #[error("erro ao interpretar resposta: {0}")]
    Parse(String),
    #[error("chave de API ausente — salve a chave em Configurações")]
    MissingApiKey,
    #[error("requisição inválida: {0}")]
    InvalidRequest(String),
}

impl From<AppError> for tauri::ipc::InvokeError {
    fn from(e: AppError) -> Self {
        tauri::ipc::InvokeError::from(e.to_string())
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Database(e.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            return Self::Network("tempo esgotado — verifique sua conexão".into());
        }
        if e.is_connect() {
            return Self::Network("sem conexão — verifique sua internet".into());
        }
        Self::Network(e.to_string())
    }
}

/// Extrai a mensagem de erro da API do Google (JSON) para diagnóstico.
pub fn google_error_hint(body: &str) -> String {
    let short: String = body.chars().take(300).collect();
    if short.contains("has not been used") || short.contains("SERVICE_DISABLED") {
        return "a Places API (New) não está ativada no Google Cloud para esta chave".into();
    }
    if short.contains("BILLING_DISABLED") || short.contains("billing") {
        return "o faturamento (billing) não está ativo no projeto Google Cloud".into();
    }
    if short.contains("API_KEY_INVALID") || short.contains("API key not valid") {
        return "a chave de API é inválida".into();
    }
    if short.contains("REQUEST_DENIED") {
        return format!("pedido negado: {short}");
    }
    short
}
