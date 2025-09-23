use std::sync::Arc;

use sea_orm::DatabaseConnection;

pub fn get_tag_group(_tgid: u64, _db: Arc<DatabaseConnection>) {}
