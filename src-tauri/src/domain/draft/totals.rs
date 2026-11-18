//! Base, IVA y total del borrador (FR-028, FR-029 y FR-031).

use crate::domain::draft::DraftLine;
use crate::domain::money::vat_amount_cents;
use crate::error::ErrorCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DraftTotals {
    pub base_cents: i64,
    pub vat_cents: i64,
    pub total_cents: i64,
    pub pending_count: i64,
    pub is_provisional: bool,
}

/// Suma las líneas completas. Las pendientes no cuentan, pero hacen el total provisional.
pub fn compute_totals(lines: &[DraftLine], vat_rate_bp: i64) -> Result<DraftTotals, ErrorCode> {
    let mut base_cents: i64 = 0;
    let mut pending_count: i64 = 0;

    for line in lines {
        match line.amount_cents()? {
            Some(amount) => {
                base_cents = base_cents
                    .checked_add(amount)
                    .ok_or(ErrorCode::AmountTooLarge)?;
            }
            None => pending_count += 1,
        }
    }

    let vat_cents = vat_amount_cents(base_cents, vat_rate_bp)?;
    let total_cents = base_cents
        .checked_add(vat_cents)
        .ok_or(ErrorCode::AmountTooLarge)?;

    Ok(DraftTotals {
        base_cents,
        vat_cents,
        total_cents,
        pending_count,
        is_provisional: pending_count > 0,
    })
}
