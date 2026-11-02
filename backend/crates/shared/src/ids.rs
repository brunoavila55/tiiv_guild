use uuid::Uuid;

/// Gera um novo id (UUID v7, ordenável por tempo).
pub fn new_id() -> Uuid {
    Uuid::now_v7()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_id_is_v7() {
        assert_eq!(new_id().get_version_num(), 7);
    }
}
