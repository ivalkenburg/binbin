use crate::{
    App,
    error::{Result, bad},
    models::{Identity, Settings, Share},
    storage::database::{all, put},
};
impl App {
    pub fn storage_usage(&self) -> Result<u64> {
        self.db(|db| {
            Ok(all::<Share>(db, "share")?
                .iter()
                .map(|share| share.reserved)
                .sum())
        })
    }
    pub fn update_settings(&self, actor: &Identity, settings: Settings) -> Result<Settings> {
        self.require_admin(actor)?;
        if settings.file_limit == 0
            || settings.share_limit == 0
            || settings.storage_limit == 0
            || settings.file_count == 0
            || settings.file_count > 10000
            || settings.default_days == Some(0)
            || settings.max_days == Some(0)
            || settings.max_downloads == Some(0)
            || settings
                .max_days
                .is_some_and(|maximum| settings.default_days.is_none_or(|days| days > maximum))
        {
            return Err(bad(
                "Settings must be positive and defaults must fit the maximums",
            ));
        }
        self.authenticated_db(actor, |db| put(db, "settings", "instance", &settings))?;
        Ok(settings)
    }
}
