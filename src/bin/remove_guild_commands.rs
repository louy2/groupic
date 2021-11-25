use tracing::info;
use twilight_model::id::{ApplicationId, GuildId};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = std::env::var("DISCORD_BOT_TOKEN").expect("Please set DISCORD_BOT_TOKEN");

    let application_id = std::env::var("DISCORD_APP_ID").expect("Please set DISCORD_APP_ID");
    let application_id = u64::from_str_radix(&application_id, 10)?;
    let application_id = ApplicationId::new(application_id)
        .ok_or(anyhow::anyhow!("Invalid application id in DISCORD_APP_ID"))?;

    let hc = twilight_http::Client::builder()
        .token(token.clone())
        .application_id(application_id)
        .build();
    let me = hc.current_user().exec().await?.model().await?;
    info!("Using Discord API as {}#{}", me.name, me.discriminator());

    let mut args = std::env::args();
    let bin_name = args.next().unwrap();
    let msg_then_usage = |msg| format!("{}\nUsage: {} <guild_id>", msg, bin_name);
    let guild_id = args
        .next()
        .expect(msg_then_usage("guild_id not found").as_str());
    let guild_id = u64::from_str_radix(guild_id.trim(), 10)
        .expect(msg_then_usage("guild_id must be a number").as_str());
    let guild_id =
        GuildId::new(guild_id).expect(msg_then_usage("guild_id must not be zero").as_str());
    let guild_commands = hc
        .get_guild_commands(guild_id)?
        .exec()
        .await?
        .models()
        .await?;

    for c in guild_commands {
        let command_id =
            c.id.expect(format!("command_id not found in command {}", c.name).as_str());
        hc.delete_guild_command(guild_id, command_id)?
            .exec()
            .await?;
    }

    let guild_commands = hc
        .get_guild_commands(guild_id)?
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
