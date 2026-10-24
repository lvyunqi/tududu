//! 管理端查询与操作。

pub mod store;

/// 用户状态只允许在正常、禁言、封禁之间切换。
pub const ALLOWED_USER_STATUS: [i16; 3] = [1, 2, 3];

pub fn is_valid_user_status(status: i16) -> bool {
    ALLOWED_USER_STATUS.contains(&status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_known_statuses_are_allowed() {
        assert!(is_valid_user_status(1));
        assert!(is_valid_user_status(3));
        assert!(!is_valid_user_status(4));
        assert!(!is_valid_user_status(0));
    }
}
