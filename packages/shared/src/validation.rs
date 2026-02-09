use crate::error::AppError;

const RESERVED_SLUGS: &[&str] = &[
    "api", "app", "www", "admin", "dashboard", "login", "s", "status", "health", "about",
    "pricing", "docs", "blog", "support", "help", "settings", "account", "signup", "register",
    "new", "create", "edit", "delete",
];

pub fn validate_slug(s: &str) -> Result<(), AppError> {
    if s.len() < 3 || s.len() > 60 {
        return Err(AppError::Validation(
            "Slug must be between 3 and 60 characters".to_string(),
        ));
    }

    if !s
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(AppError::Validation(
            "Slug must contain only lowercase letters, numbers, and hyphens".to_string(),
        ));
    }

    if s.starts_with('-') || s.ends_with('-') {
        return Err(AppError::Validation(
            "Slug must not start or end with a hyphen".to_string(),
        ));
    }

    if s.contains("--") {
        return Err(AppError::Validation(
            "Slug must not contain consecutive hyphens".to_string(),
        ));
    }

    if RESERVED_SLUGS.contains(&s) {
        return Err(AppError::Validation(format!(
            "Slug '{}' is reserved",
            s
        )));
    }

    Ok(())
}

pub fn validate_brand_color(s: &str) -> Result<(), AppError> {
    if s.len() != 7 {
        return Err(AppError::Validation(
            "Brand color must be in #RRGGBB format".to_string(),
        ));
    }

    if !s.starts_with('#') {
        return Err(AppError::Validation(
            "Brand color must start with #".to_string(),
        ));
    }

    if !s[1..].chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(AppError::Validation(
            "Brand color must contain valid hex digits".to_string(),
        ));
    }

    Ok(())
}

pub fn validate_org_name(s: &str) -> Result<(), AppError> {
    let trimmed = s.trim();
    if trimmed.is_empty() || trimmed.len() > 255 {
        return Err(AppError::Validation(
            "Organization name must be between 1 and 255 characters".to_string(),
        ));
    }
    Ok(())
}

/// Generate a slug from an organization name.
pub fn slugify(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_slugs() {
        assert!(validate_slug("my-org").is_ok());
        assert!(validate_slug("acme-corp-123").is_ok());
        assert!(validate_slug("abc").is_ok());
    }

    #[test]
    fn test_invalid_slugs() {
        assert!(validate_slug("ab").is_err()); // too short
        assert!(validate_slug("-my-org").is_err()); // starts with hyphen
        assert!(validate_slug("my-org-").is_err()); // ends with hyphen
        assert!(validate_slug("my--org").is_err()); // consecutive hyphens
        assert!(validate_slug("My-Org").is_err()); // uppercase
        assert!(validate_slug("my org").is_err()); // space
        assert!(validate_slug("api").is_err()); // reserved
        assert!(validate_slug("dashboard").is_err()); // reserved
    }

    #[test]
    fn test_valid_brand_colors() {
        assert!(validate_brand_color("#3B82F6").is_ok());
        assert!(validate_brand_color("#000000").is_ok());
        assert!(validate_brand_color("#FFFFFF").is_ok());
    }

    #[test]
    fn test_invalid_brand_colors() {
        assert!(validate_brand_color("3B82F6").is_err()); // no #
        assert!(validate_brand_color("#3B82F").is_err()); // too short
        assert!(validate_brand_color("#3B82F6F").is_err()); // too long
        assert!(validate_brand_color("#GGGGGG").is_err()); // invalid hex
    }

    #[test]
    fn test_valid_org_names() {
        assert!(validate_org_name("Acme Corp").is_ok());
        assert!(validate_org_name("A").is_ok());
    }

    #[test]
    fn test_invalid_org_names() {
        assert!(validate_org_name("").is_err());
        assert!(validate_org_name("   ").is_err());
    }

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Acme Corp"), "acme-corp");
        assert_eq!(slugify("My  Awesome  Org"), "my-awesome-org");
        assert_eq!(slugify("hello_world"), "hello-world");
    }
}
