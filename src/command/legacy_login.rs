use crate::{aws, config::Config};

pub fn login_to_legacy_aws_account(config: &Config) {
    aws::legacy_login(config.op_legacy_item_id.as_deref());
}
