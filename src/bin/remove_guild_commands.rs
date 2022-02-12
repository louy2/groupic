use tracing::info;
use twilight_model::id::{Id, marker::{ApplicationMarker, GuildMarker}};

type ApplicationId = Id<ApplicationMarker>;
type GuildId = Id<GuildMarker>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = std::env::var("DISCORD_BOT_TOKEN").expect("Please set DISCORD_BOT_TOKEN");

    let application_id = std::env::var("DISCORD_APP_ID").expect("Please set DISCORD_APP_ID");
    let application_id = application_id.parse::<u64>()?;
    let application_id = ApplicationId::new_checked(application_id)
        .ok_or(anyhow::anyhow!("Invalid application id in DISCORD_APP_ID"))?;

    let hc = twilight_http::Client::builder()
        .token(token.clone())
        .build();
    let me = hc.current_user().exec().await?.model().await?;
    info!("Using Discord API as {}#{}", me.name, me.discriminator());
    let ic = hc.interaction(application_id);

    let mut args = std::env::args();
    let bin_name = args.next().unwrap();
    let msg_then_usage = |msg| format!("{}\nUsage: {} <guild_id>", msg, bin_name);
    let guild_id = args
        .next()
        .unwrap_or_else(|| { panic!("{}", msg_then_usage("guild_id not found")) });
    let guild_id = guild_id.trim().parse::<u64>()
        .unwrap_or_else(|_| { panic!("{}", msg_then_usage("guild_id must be a number")) });
    let guild_id =
        GuildId::new_checked(guild_id).unwrap_or_else(|| { panic!("{}", msg_then_usage("guild_id must not be zero")) });
    let guild_commands = ic
        .get_guild_commands(guild_id)
        .exec()
        .await?
        .models()
        .await?;

    for c in guild_commands {
        let command_id =
            c.id.unwrap_or_else(|| panic!("command_id not found in command {}", c.name));
        ic.delete_guild_command(guild_id, command_id)
            .exec()
            .await?;
    }

    let guild_commands = ic
        .get_guild_commands(guild_id)
        .exec()
        .await?
        .models()
        .await?;
    if !guild_commands.is_empty() {
        panic!("Failed to delete all guild commands.");
    }

    println!("Succeeded in deleting all guild commands.");

    Ok(())
}
