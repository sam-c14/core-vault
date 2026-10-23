use axum::{extract::Path, http::StatusCode, routing::get, Json, Router};
use cv_platform_config_parser::PlatformConfigParser;
use serde_json::{json, Value};
use std::path::PathBuf;

pub fn router() -> Router {
    Router::new()
        .route("/api/v1/config/jurisdictions", get(list_jurisdictions))
        .route(
            "/api/v1/config/jurisdictions/:code",
            get(get_jurisdiction_config),
        )
}

async fn list_jurisdictions() -> Json<Value> {
    Json(json!({
        "status": "SUCCESS",
        "supported_jurisdictions": [
            {
                "country_code": "NG",
                "name": "Nigeria",
                "currency": "NGN",
                "account_standard": "NUBAN",
                "regulatory_body": "Central Bank of Nigeria (CBN)"
            },
            {
                "country_code": "GB",
                "name": "United Kingdom",
                "currency": "GBP",
                "account_standard": "SORT_CODE_AND_ACCOUNT",
                "regulatory_body": "Financial Conduct Authority (FCA)"
            }
        ]
    }))
}

async fn get_jurisdiction_config(
    Path(code): Path<String>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    let clean_code = code.to_uppercase();
    let file_path = PathBuf::from(format!("config/jurisdictions/{}.toml", clean_code));

    if !file_path.exists() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "status": "NOT_FOUND",
                "message": format!("Jurisdiction configuration '{}' not found", clean_code)
            })),
        ));
    }

    match PlatformConfigParser::parse_file(&file_path) {
        Ok(cfg) => Ok((
            StatusCode::OK,
            Json(json!({
                "status": "SUCCESS",
                "configuration": cfg
            })),
        )),
        Err(err) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "status": "PARSE_ERROR",
                "message": err.to_string()
            })),
        )),
    }
}
