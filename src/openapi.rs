use utoipa::{
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    Modify, OpenApi,
};

use crate::controllers::{
    admin_users, audit_log, auth, exports, health, me, notifications, record_types, records, rules,
};

/// Single source of truth for the API contract. Add each new endpoint to `paths` and each
/// request/response type to `schemas`. The committed `openapi.json` is checked by a test and by CI;
/// the frontend generates its TypeScript types from it (`npm run gen:api`).
#[derive(OpenApi)]
#[openapi(
    info(title = "Brightcone People API", version = "0.1.0"),
    paths(
        health::health,
        auth::login,
        auth::refresh,
        auth::logout,
        auth::forgot,
        auth::reset,
        me::current,
        me::change_password,
        admin_users::list,
        admin_users::create,
        admin_users::unlock,
        admin_users::disable,
        admin_users::set_role,
        admin_users::reset_password,
        audit_log::list,
        notifications::list,
        notifications::mark_read,
        records::list,
        records::create,
        records::update,
        records::archive,
        records::restore,
        records::history,
        record_types::list,
        exports::csv,
        exports::backup,
        exports::import,
        rules::list,
        rules::metrics,
        rules::retry,
        rules::update,
    ),
    components(schemas(
        crate::views::health::HealthResponse,
        crate::views::auth::LoginParams,
        crate::views::auth::ForgotParams,
        crate::views::auth::ResetParams,
        crate::views::auth::ChangePasswordParams,
        crate::views::auth::MeResponse,
        crate::views::auth::SessionResponse,
        crate::views::auth::MessageResponse,
        admin_users::AdminUserResponse,
        admin_users::CreateUserParams,
        admin_users::SetRoleParams,
        admin_users::TemporaryPasswordResponse,
        audit_log::AuditEntryResponse,
        audit_log::AuditPage,
        notifications::Notification,
        notifications::NotificationPage,
        records::RecordInput,
        records::RecordUpdateInput,
        record_types::FieldSchema,
        record_types::RecordTypeSchema,
        record_types::RecordCatalog,
        records::Record,
        records::RecordPage,
        records::HistoryEntry,
        exports::BackupDocument,
        exports::BackupRecord,
        exports::BackupHistory,
        exports::BackupImport,
        rules::Rule,
        rules::RuleUpdate,
        rules::RuleMetrics,
    )),
    modifiers(&BearerAuth)
)]
pub struct ApiDoc;

struct BearerAuth;

impl Modify for BearerAuth {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
        }
    }
}
