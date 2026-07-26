use nonstick::{ConversationAdapter, Result as PamResult};
use tracing::{info, warn};

/// A basic Conversation that assumes that any "regular" prompt is for
/// the username, and that any "masked" prompt is for the password.
///
/// A typical Conversation will provide the user with an interface
/// to interact with PAM, e.g. a dialogue box or a terminal prompt.
pub(crate) struct UsernamePassConvo {
    pub(crate) username: String,
    pub(crate) password: String,
}

// ConversationAdapter is a convenience wrapper for the common case
// of only handling one request at a time.
impl ConversationAdapter for UsernamePassConvo {
    fn prompt(&self, request: impl AsRef<std::ffi::OsStr>) -> PamResult<std::ffi::OsString> {
        info!("Request: {:?}", request.as_ref());
        Ok(std::ffi::OsString::from(&self.username))
    }

    fn masked_prompt(&self, request: impl AsRef<std::ffi::OsStr>) -> PamResult<std::ffi::OsString> {
        info!("Masked Request: {:?}", request.as_ref());
        Ok(std::ffi::OsString::from(&self.password))
    }

    fn error_msg(&self, message: impl AsRef<std::ffi::OsStr>) {
        warn!("Ignored Error Message: {:?}", message.as_ref());
    }

    fn info_msg(&self, message: impl AsRef<std::ffi::OsStr>) {
        warn!("Ignored Info Message: {:?}", message.as_ref());
    }
}

pub(crate) struct FingerprintInfo;

impl ConversationAdapter for FingerprintInfo {
    fn prompt(&self, request: impl AsRef<std::ffi::OsStr>) -> PamResult<std::ffi::OsString> {
        warn!("Ignored Prompt: {:?}", request.as_ref());
        Ok(std::ffi::OsString::from(""))
    }

    fn masked_prompt(&self, request: impl AsRef<std::ffi::OsStr>) -> PamResult<std::ffi::OsString> {
        warn!("Ignored masked prompt: {:?}", request.as_ref());
        Ok(std::ffi::OsString::from(""))
    }

    fn info_msg(&self, message: impl AsRef<std::ffi::OsStr>) {
        info!("Info Message: {:?}", message.as_ref());
    }

    fn error_msg(&self, message: impl AsRef<std::ffi::OsStr>) {
        info!("Error Message: {:?}", message.as_ref());
    }
}
