use axum::{http::StatusCode, routing::post, Json, Router};
use cv_double_entry_engine::{validate_journal_batch, PostingLeg};
use cv_types::{Currency, EntryType};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

pub fn router() -> Router {
    Router::new().route("/api/v1/transfers/intra", post(post_intra_transfer))
}

#[derive(Debug, Deserialize, Serialize)]
pub struct IntraTransferDto {
    pub source_account_id: Uuid,
    pub destination_account_id: Uuid,
    pub amount: Decimal,
    pub currency: Currency,
    pub narration: String,
}

async fn post_intra_transfer(
    Json(dto): Json<IntraTransferDto>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    let legs = vec![
        PostingLeg {
            account_id: dto.source_account_id,
            gl_code: "2110001".to_string(),
            currency: dto.currency,
            entry_type: EntryType::Debit,
            amount: dto.amount,
        },
        PostingLeg {
            account_id: dto.destination_account_id,
            gl_code: "2110002".to_string(),
            currency: dto.currency,
            entry_type: EntryType::Credit,
            amount: dto.amount,
        },
    ];

    if let Err(err) = validate_journal_batch(&legs) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "status": "VALIDATION_FAILED",
                "error": err.to_string()
            })),
        ));
    }

    let txn_id = Uuid::new_v4();
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "status": "COMMITTED",
            "transaction_id": txn_id,
            "amount": dto.amount,
            "currency": dto.currency,
            "narration": dto.narration
        })),
    ))
}
