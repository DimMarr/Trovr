use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedUser {
    pub user_id: Uuid,
    pub issuer: String,
    pub subject: String,
    pub email: String,
    pub display_name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn users_with_the_same_fields_are_equal() {
        let user_id = Uuid::nil();
        let a = AuthenticatedUser {
            user_id,
            issuer: "internal".to_string(),
            subject: "abc".to_string(),
            email: "a@example.com".to_string(),
            display_name: "A".to_string(),
        };
        let b = a.clone();
        assert_eq!(a, b);
    }
}
