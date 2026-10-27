use loco_rs::{environment::Environment, hash, prelude::*};

use crate::{
    authz::Role,
    database::Database,
    models::{
        error::ModelError,
        refresh_tokens,
        users::{self, NewUser},
    },
};

/// Demo accounts for development. Replaces the prototype's `seed()`; employee records, departments
/// and leave data are added to this task by the Phase 2 and 3 tickets.
pub struct SeedDemo;

const PEOPLE: &[(&str, Role)] = &[
    ("Ananya Rao", Role::HrAdmin),
    ("Naresh Vemparala", Role::Manager),
    ("Karthik Reddy", Role::Manager),
    ("Priya Menon", Role::Manager),
    ("Meera Joshi", Role::Finance),
    ("Rahul Varma", Role::Employee),
    ("Sneha Iyer", Role::Employee),
    ("Arjun Nair", Role::Employee),
    ("Daniel Brooks", Role::Employee),
    ("Emily Carter", Role::Employee),
    ("Vikram Singh", Role::Employee),
    ("Lakshmi Prasad", Role::Employee),
    ("Farhan Ali", Role::Employee),
    ("Divya Kulkarni", Role::Employee),
];

/// `Ananya Rao` becomes `ananya.rao@brightcone.ai`.
fn email_for(name: &str) -> String {
    let local: Vec<String> = name
        .to_lowercase()
        .split_whitespace()
        .map(|part| part.chars().filter(char::is_ascii_alphabetic).collect())
        .collect();
    format!("{}@brightcone.ai", local.join("."))
}

#[async_trait]
impl Task for SeedDemo {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "seed_demo".to_string(),
            detail: "Create demo accounts with passwords printed once. Refuses to run in production.\nUsage: cargo loco task seed_demo [email:ananya.rao@brightcone.ai]"
                .to_string(),
        }
    }

    async fn run(&self, ctx: &AppContext, vars: &task::Vars) -> Result<()> {
        if ctx.environment == Environment::Production {
            return Err(Error::string("seed_demo must never run in production"));
        }
        let only_email = vars.cli_arg("email").ok().map(str::to_lowercase);
        if let Some(email) = &only_email {
            if !PEOPLE.iter().any(|(name, _)| email_for(name) == *email) {
                return Err(Error::string(
                    "email must match one of the configured demo accounts",
                ));
            }
        }
        let db = Database::from_context(ctx)?;
        for (name, role) in PEOPLE {
            let email = email_for(name);
            if only_email.as_deref().is_some_and(|target| target != email) {
                continue;
            }
            let known_password = match email.as_str() {
                "ananya.rao@brightcone.ai" => "1UGS09mAAVEwRPw4-Aa1",
                "meera.joshi@brightcone.ai" => "4mfFhauy0F3Y9wRS-Aa1",
                "rahul.varma@brightcone.ai" => "66nLUcOI4YO4hfDn-Aa1",
                _ => "Password@123",
            };
            match users::Model::find_by_email(&db, &email).await {
                Ok(existing) => {
                    let updated = existing
                        .into_active_model()
                        .set_password(&db, known_password, false)
                        .await
                        .map_err(|e| Error::string(&format!("{e:?}")))?;
                    let mut active = updated.into_active_model();
                    active.role = ActiveValue::set(role.as_str().to_owned());
                    active.status = ActiveValue::set("active".to_owned());
                    active.failed_logins = ActiveValue::set(0);
                    active.locked_at = ActiveValue::set(None);
                    let updated = active.update(&db).await?;
                    refresh_tokens::Model::revoke_all_for_user(&db, updated.id).await?;
                    println!("updated {email:<34} password: {known_password}");
                }
                Err(ModelError::EntityNotFound) => {
                    let password = if email == "ananya.rao@brightcone.ai" {
                        known_password.to_owned()
                    } else {
                        format!("{}-Aa1", hash::random_string(16))
                    };
                    users::Model::create_with_password(
                        &db,
                        &NewUser {
                            email: email.clone(),
                            name: (*name).to_owned(),
                            password: password.clone(),
                            role: *role,
                            must_change_password: false,
                        },
                    )
                    .await?;
                    println!("created {email:<34} {:<9} {password}", role.as_str());
                }
                Err(error) => return Err(Error::string(&format!("{error}"))),
            }
        }
        println!("\nThese passwords are shown once.");
        Ok(())
    }
}
