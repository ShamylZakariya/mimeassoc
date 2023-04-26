pub struct AppId {
    pub id: String,
}

impl AppId {
    pub fn parse(id: &str) -> anyhow::Result<AppId> {
        if id.ends_with(".desktop") {
            Ok(AppId { id: id.to_string() })
        } else {
            anyhow::bail!("id: \"{}\" not a valid desktop entry", id)
        }
    }
}
