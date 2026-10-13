use loco_rs::prelude::*;

pub use super::_entities::audit_logs::{self, ActiveModel, Entity, Model};

impl ActiveModelBehavior for ActiveModel {}
