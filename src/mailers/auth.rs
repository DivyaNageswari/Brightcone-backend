// auth mailer
#![allow(non_upper_case_globals)]

use loco_rs::prelude::*;
use serde_json::json;

use crate::{models::users, settings::AppSettings};

static forgot: Dir<'_> = include_dir!("src/mailers/auth/forgot");

#[allow(clippy::module_name_repetitions)]
pub struct AuthMailer {}
impl Mailer for AuthMailer {}
impl AuthMailer {
    /// Sends the password-reset link. `secret` is the one-time token; only its hash is stored.
    ///
    /// # Errors
    /// When the email cannot be queued or sent.
    pub async fn send_password_reset(
        ctx: &AppContext,
        user: &users::Model,
        secret: &str,
    ) -> Result<()> {
        let frontend = AppSettings::from_ctx(ctx).frontend_url;
        Self::mail_template(
            ctx,
            &forgot,
            mailer::Args {
                to: user.email.clone(),
                locals: json!({
                  "name": user.name,
                  "resetUrl": format!("{}/reset-password?token={secret}", frontend.trim_end_matches('/')),
                }),
                ..Default::default()
            },
        )
        .await?;
        Ok(())
    }
}
