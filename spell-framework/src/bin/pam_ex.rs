fn main() {
    use pam_client::conv_mock::Conversation;
    use pam_client::{Context, Flag}; // Non-interactive implementation

    let mut context = Context::new(
        "login", // Service name
        None,
        Conversation::with_credentials("ramayen", "Bhau@07"),
    )
    .expect("Failed to initialize PAM context");

    // Authenticate the user
    context
        .authenticate(Flag::NONE)
        .expect("Authentication failed");

    // Validate the account
    context
        .acct_mgmt(Flag::NONE)
        .expect("Account validation failed");
    // let mut context = Context::new(
    //     "login",             // Service name, decides which policy is used (see `/etc/pam.d`)
    //     None,                // Optional preset user name
    //     Conversation::new(), // Handler for user interaction
    // )
    // .expect("Failed to initialize PAM context");
    //
    // // Optionally set some settings
    // context.set_user_prompt(Some("Who art thou? "));
    //
    // // Authenticate the user (ask for password, 2nd-factor token, fingerprint, etc.)
    // context
    //     .authenticate(Flag::NONE)
    //     .expect("Authentication failed");
    //
    // // Validate the account (is not locked, expired, etc.)
    // context
    //     .acct_mgmt(Flag::NONE)
    //     .expect("Account validation failed");
    //
    // // Get resulting user name and map to a user id
    // let username = context.user();
    // println!("{}", username.unwrap());
    // let uid = 65535; // Left as an exercise to the reader
    //
    // // Open session and initialize credentials
    // let mut session = context
    //     .open_session(Flag::NONE)
    //     .expect("Session opening failed");
    //
    // // Run a process in the PAM environment
    // let result = Command::new("/usr/bin/some_program")
    //     .env_clear()
    //     .envs(session.envlist().iter_tuples())
    //     .uid(uid)
    //     // .gid(...)
    //     .status();
    //
    // The session is automatically closed when it goes out of scope.
}
