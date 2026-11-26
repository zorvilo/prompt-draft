use std::fmt;
/// IPC errors remain plain strings. Only user-facing messages cross this boundary.
#[derive(Debug, serde::Serialize)]
#[serde(transparent)]
pub struct AppError(String);
pub type CommandResult<T> = Result<T, AppError>;
impl From<String> for AppError {
    fn from(message: String) -> Self {
        Self(message)
    }
}
impl From<&str> for AppError {
    fn from(message: &str) -> Self {
        Self(message.to_owned())
    }
}
impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for AppError {}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ipc_errors_keep_the_existing_string_contract() {
        for error in [
            AppError::from("操作失败。"),
            AppError::from(String::from("已取消优化。")),
        ] {
            assert_eq!(
                serde_json::to_value(&error).unwrap(),
                serde_json::Value::String(error.to_string())
            );
        }
    }
}
