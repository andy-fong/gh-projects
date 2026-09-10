use serde::{Deserialize, Serialize};

/// The groups a login can be filed under. Anyone absent from the roster is
/// classified `community` by the frontend, so it is not a stored value.
pub const MEMBER_GROUPS: [&str; 5] = ["team", "pe", "solo", "maintainer", "bot"];

/// A GitHub login on the roster, used to tag item authors in GH Query tiles.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TeamMember {
    pub id: i64,
    pub login: String,
    pub member_group: String,
    /// `manual`, or the `org_team` of the source it was imported from.
    pub source: String,
    pub position: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateTeamMemberInput {
    pub login: String,
    pub member_group: String,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTeamMemberInput {
    pub login: Option<String>,
    pub member_group: Option<String>,
    pub position: Option<i64>,
}

/// A GitHub team or org whose members can be pulled into a group on demand.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TeamMemberSource {
    pub id: i64,
    pub member_group: String,
    /// `owner/team-slug` for a team, or a bare `owner` for a whole org.
    pub org_team: String,
    pub position: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateTeamMemberSourceInput {
    pub member_group: String,
    pub org_team: String,
}

/// Outcome of re-importing every configured source.
#[derive(Debug, Default, Serialize)]
pub struct RefreshResult {
    pub added: usize,
    /// Logins already on the roster; their existing group is left alone.
    pub skipped: usize,
    pub errors: Vec<String>,
}
