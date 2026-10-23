use cv_types::{Currency, EntryType};
use rust_decimal::Decimal;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostingLeg {
    pub account_id: Uuid,
    pub gl_code: String,
    pub currency: Currency,
    pub entry_type: EntryType,
    pub amount: Decimal,
}

#[derive(Debug, Clone)]
pub struct JournalBatch {
    pub transaction_id: Uuid,
    pub narration: String,
    pub legs: Vec<PostingLeg>,
}

impl JournalBatch {
    pub fn new(transaction_id: Uuid, narration: String) -> Self {
        Self {
            transaction_id,
            narration,
            legs: Vec::new(),
        }
    }

    pub fn add_leg(&mut self, leg: PostingLeg) {
        self.legs.push(leg);
    }
}
