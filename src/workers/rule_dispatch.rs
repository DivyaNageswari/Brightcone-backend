use async_trait::async_trait;
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::rule_engine;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleDispatchArgs {
    pub event_id: Uuid,
}

pub struct RuleDispatchWorker {
    ctx: AppContext,
}

#[async_trait]
impl BackgroundWorker<RuleDispatchArgs> for RuleDispatchWorker {
    fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
    }

    async fn perform(&self, args: RuleDispatchArgs) -> Result<()> {
        for retry in 0..5 {
            match rule_engine::process_event(&self.ctx, args.event_id).await {
                Ok(()) => return Ok(()),
                Err(error) => {
                    if let Err(mark_error) =
                        rule_engine::record_failure(&self.ctx, args.event_id).await
                    {
                        tracing::error!(event_id = %args.event_id, error = %mark_error, "could not record rule failure");
                    }
                    if retry == 4 {
                        return Err(error);
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(200 * (1 << retry))).await;
                }
            }
        }
        Ok(())
    }
}
