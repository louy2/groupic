use twilight_http::Client;
use twilight_model::id::GuildId;

#[macro_export]
macro_rules! dbg_debug {
    ($var:expr) => {{
        tracing::debug!(concat!(stringify!($var), " = {:?}"), $var);
        $var
    }};
}

#[macro_export]
macro_rules! dbg_trace {
    ($var:expr) => {{
        tracing::trace!(concat!(stringify!($var), " = {:?}"), $var);
        $var
    }};
}

#[allow(dead_code)]
pub async fn delete_guild_commands(client: &Client, guild_id: GuildId) -> anyhow::Result<()> {
    let guild_commands = client
        .get_guild_commands(guild_id)?
        .exec().await?.models().await?;
    for c in guild_commands {
        if let Some(command_id) = c.id {
            client.delete_guild_command(guild_id, command_id)?.exec().await?;
        }
    }
    Ok(())
}

pub mod cdn {
    use twilight_model::id::{GuildId, UserId};

    #[allow(dead_code)]
    pub enum PJWG {
        PNG,
        JPEG,
        WebP,
        GIF,
    }

    impl AsRef<str> for PJWG {
        fn as_ref(&self) -> &'static str {
            match self {
                Self::PNG => "png",
                Self::JPEG => "jpeg",
                Self::WebP => "webp",
                Self::GIF => "gif",
            }
        }
    }

    /// Join path with Discord CDN base url
    #[macro_export]
    macro_rules! base {
        ($path:expr) => {
            format!("https://cdn.discordapp.com/{}", $path)
        };
    }

    /// Get path to default user avatar by user discriminator
    macro_rules! default_user_avatar {
        ($user_discriminator:expr) => {
            format!("embed/avatars/{}.png", $user_discriminator % 5)
        };
    }

    /// Get path to user avatar by user id, user avatar hash, and image format
    macro_rules! user_avatar {
        ($user_id:expr, $user_avatar:expr, $format:expr) => {
            format!("avatars/{}/{}.{}", $user_id, $user_avatar, $format)
        };
    }

    /// Get path to guild member avatar by guild id, user id, member avatar hash, and image format
    macro_rules! guild_member_avatar {
        ($guild_id:expr, $user_id:expr, $member_avatar:expr, $format:expr) => {
            format!(
                "guilds/{}/users/{}/avatars/{}.{}",
                $guild_id, $user_id, $member_avatar, $format
            )
        };
    }

    pub fn get_default_user_avatar(discriminator: u16) -> String {
        base!(default_user_avatar!(discriminator))
    }

    pub fn get_user_avatar<S>(user_id: UserId, user_avatar: S, format: PJWG) -> String
    where
        S: AsRef<str>,
    {
        base!(user_avatar!(
            user_id.0,
            user_avatar.as_ref(),
            format.as_ref()
        ))
    }

    pub fn get_guild_member_avatar<S>(
        guild_id: GuildId,
        user_id: UserId,
        member_avatar: S,
        format: PJWG,
    ) -> String
    where
        S: AsRef<str>,
    {
        base!(guild_member_avatar!(
            guild_id.0,
            user_id.0,
            member_avatar.as_ref(),
            format.as_ref()
        ))
    }
}
