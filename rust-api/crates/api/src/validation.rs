use garde::Report;
use kernel::{AppError, ValidationIssue};

/// Flattens a `garde::Report` into the API's field-level `details` shape.
pub fn to_app_error(report: Report) -> AppError {
    let issues: Vec<ValidationIssue> = report
        .iter()
        .map(|(path, err)| ValidationIssue::new(path.to_string(), err.to_string()))
        .collect();

    AppError::validation_with_issues(
        "validation/invalid-argument",
        "request validation failed",
        issues,
    )
}
