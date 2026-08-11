use kernel::AppError;

/// Integer minor units plus a currency code — never a float on this path.
/// `price_cents` is the smallest unit (e.g. USD cents); `currency` is a
/// 3-letter uppercase ISO-4217 code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Money {
    price_cents: i64,
    currency: String,
}

impl Money {
    pub fn new(price_cents: i64, currency: impl AsRef<str>) -> Result<Self, AppError> {
        if price_cents < 0 {
            return Err(AppError::validation(
                "product/invalid-price",
                "price_cents must be non-negative",
            ));
        }

        let currency = currency.as_ref().trim().to_string();
        let is_valid_currency =
            currency.len() == 3 && currency.chars().all(|c| c.is_ascii_uppercase());
        if !is_valid_currency {
            return Err(AppError::validation(
                "product/invalid-currency",
                "currency must be a 3-letter uppercase ISO-4217 code",
            ));
        }

        Ok(Self {
            price_cents,
            currency,
        })
    }

    pub fn price_cents(&self) -> i64 {
        self.price_cents
    }

    pub fn currency(&self) -> &str {
        &self.currency
    }
}
