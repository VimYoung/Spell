use nonstick::{
    AuthnFlags, Conversation, ConversationAdapter, Result as PamResult, Transaction,
    TransactionBuilder,
};
use std::ffi::{OsStr, OsString};

/// A basic Conversation that assumes that any "regular" prompt is for
/// the username, and that any "masked" prompt is for the password.
///
/// A typical Conversation will provide the user with an interface
/// to interact with PAM, e.g. a dialogue box or a terminal prompt.
struct UsernamePassConvo {
    username: String,
    password: String,
}

// ConversationAdapter is a convenience wrapper for the common case
// of only handling one request at a time.
impl ConversationAdapter for UsernamePassConvo {
    fn prompt(&self, request: impl AsRef<OsStr>) -> PamResult<OsString> {
        Ok(OsString::from(&self.username))
    }

    fn masked_prompt(&self, request: impl AsRef<OsStr>) -> PamResult<OsString> {
        Ok(OsString::from(&self.password))
    }

    fn error_msg(&self, message: impl AsRef<OsStr>) {
        // Normally you would want to display this to the user somehow.
        // In this case, we're just ignoring it.
    }

    fn info_msg(&self, message: impl AsRef<OsStr>) {
        // ibid.
    }
}

fn main() -> PamResult<()> {
    let username: &str = "rgmayen";
    let password: &str = "djfvfkdv7";
    let user_pass = UsernamePassConvo {
        username: username.into(),
        password: password.into(),
    };

    let mut txn = TransactionBuilder::new_with_service("login")
        .username(username)
        .build(user_pass.into_conversation())?;
    // If authentication fails, this will return an error.
    // We immediately give up rather than re-prompting the user.
    txn.authenticate(AuthnFlags::empty())?;
    txn.account_management(AuthnFlags::empty())?;
    Ok(())
}
