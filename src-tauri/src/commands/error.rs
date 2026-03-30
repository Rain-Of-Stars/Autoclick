use autoclick_diagnostics::error_code::ErrorCode;
use autoclick_domain::user_message::UserMessage;

pub type CommandResult<T> = Result<T, UserMessage>;

pub fn command_error(code: ErrorCode, detail: impl Into<String>) -> UserMessage {
    code.to_user_message(detail)
}
