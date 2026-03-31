//! Email login extension: email registration, magic link login, token redemption.

mod email;
mod operations;

pub use email::{generate_token, send_login_email};
pub use operations::EmailHandler;
