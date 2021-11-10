mod alias;
mod gen_pic;
mod util;

use anyhow::Context;
use futures::future::try_join_all;
use std::num::NonZeroU64;
use std::str::FromStr;
use tempdir::TempDir;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::task::spawn_blocking;
use tokio_stream::StreamExt;
use tracing::{error, info};

use alias::*;
use util::*;

use twilight_cache_inmemory::{InMemoryCache, ResourceType};
use twilight_gateway::{Event, EventTypeFlags, Intents, Shard};
use twilight_model::application::callback::InteractionResponse;
use twilight_model::application::command::{ChannelCommandOptionData, Command, CommandOption};
use twilight_model::application::interaction::application_command::CommandOptionValue;
use twilight_model::application::interaction::Interaction;
use twilight_model::channel::message::MessageFlags;
use twilight_model::channel::{Channel, ChannelType, GuildChannel};
use twilight_model::guild::Member;
use twilight_model::id::{ApplicationId, GuildId};

lazy_static::lazy_static! {
    static ref TEST_GUILD_ID: GuildId = GuildId(NonZeroU64::new(137463604311097345).unwrap());
    static ref APPLICATION_ID: ApplicationId = ApplicationId(NonZeroU64::new(794225841554325516).unwrap());
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // set up global trace collector
    #[cfg(debug_assertions)]
    tracing_subscriber::fmt()
        .pretty()
        .with_thread_names(true)
        .with_max_level(tracing::Level::DEBUG)
        .init();
    #[cfg(not(debug_assertions))]
    tracing_subscriber::fmt()
        .with_thread_names(true)
        .with_max_level(tracing::Level::INFO)
        .init();

    // Login with a bot token from the environment
    let token = std::env::var("DISCORD_TOKEN").expect("Please set DISCORD_TOKEN");

    let hc = twilight_http::Client::builder()
        .token(token.clone())
        .application_id(*APPLICATION_ID)
        .build();
    let me = hc.current_user().exec().await?.model().await?;
    info!("Using Discord API as {}#{}", me.name, me.discriminator());

    let ping_command: Command = hc
        .create_guild_command(*TEST_GUILD_ID, "ping")?
        .chat_input("Replies with pong")?
        .exec()
        .await?
        .model()
        .await?;
    let avatar_command: Command = hc
        .create_guild_command(*TEST_GUILD_ID, "avatar")?
        .chat_input("Replies with your avatar")?
        .exec()
        .await?
        .model()
        .await?;
    let groupic_command: Command = hc
        .create_guild_command(*TEST_GUILD_ID, "groupic")?
        .chat_input("Replies with a group picture of the given voice channel")?
        .command_options(&[CommandOption::Channel(ChannelCommandOptionData {
            channel_types: vec![ChannelType::GuildVoice],
            description: "The voice channel for group picture".into(),
            name: "channel".into(),
            required: true,
        })])?
        .exec()
        .await?
        .model()
        .await?;

    let (gc, mut events) = Shard::builder(
        token.clone(),
        Intents::GUILDS
            | Intents::GUILD_MESSAGES
            | Intents::GUILD_MESSAGE_REACTIONS
            | Intents::GUILD_VOICE_STATES,
    )
    .event_types(
        EventTypeFlags::GUILDS
            | EventTypeFlags::INTERACTION_CREATE
            | EventTypeFlags::GUILD_VOICE_STATES
            | EventTypeFlags::VOICE_STATE_UPDATE,
    )
    .build();
    gc.start().await?;

    let cache = InMemoryCache::builder()
        .resource_types(ResourceType::GUILD | ResourceType::VOICE_STATE)
        .build();

    while let Some(event) = events.next().await {
        cache.update(&event);
        match event {
            Event::Ready(x) => {
                let me = x.user;
                info!(
                    "Connecting to Discord Gateway as {}#{}",
                    me.name,
                    me.discriminator()
                );
            }
            Event::InteractionCreate(x) => {
                let x = x.0;
                match x {
                    Interaction::ApplicationCommand(ac) => {
                        // dispatch to ping
                        if ac.data.id == ping_command.id.unwrap() {
                            let res = twilight_util::builder::CallbackDataBuilder::new()
                                .content("Pong".into())
                                .flags(MessageFlags::EPHEMERAL)
                                .build();
                            hc.create_interaction_original(
                                ac.id,
                                &ac.token,
                                &InteractionResponse::ChannelMessageWithSource(res),
                            )
                            .exec()
                            .await?;
                        }
                        // dispatch to avatar
                        if ac.data.id == avatar_command.id.unwrap() {
                            let avatar_url: String = match ac.member {
                                Some(m) => match m.avatar {
                                    // get guild member avatar if exists
                                    Some(member_avatar) => {
                                        if !matches!((&ac.guild_id, &m.user), (Some(_), Some(_))) {
                                            error!("Gateway event INTERACTION_CREATE should have guild_id and member.user but doesn't");
                                            continue;
                                        } else {
                                            cdn::get_guild_member_avatar(
                                                ac.guild_id.unwrap(),
                                                m.user.unwrap().id,
                                                member_avatar,
                                                cdn::PJWG::PNG,
                                            )
                                        }
                                    }
                                    // get user avatar otherwise
                                    None => match m.user {
                                        // get user avatar if exists
                                        Some(u) => match u.avatar {
                                            Some(user_avatar) => cdn::get_user_avatar(
                                                u.id,
                                                user_avatar,
                                                cdn::PJWG::PNG,
                                            ),
                                            None => cdn::get_default_user_avatar(u.discriminator),
                                        },
                                        // get default avatar otherwise
                                        None => {
                                            error!("Gateway event INTERACTION_CREATE should have member.user but doesn't");
                                            continue;
                                        }
                                    },
                                },
                                None => {
                                    let u = ac.user.unwrap();
                                    match u.avatar {
                                        Some(user_avatar) => {
                                            cdn::get_user_avatar(u.id, user_avatar, cdn::PJWG::PNG)
                                        }
                                        None => cdn::get_default_user_avatar(u.discriminator),
                                    }
                                }
                            };
                            let res = twilight_util::builder::CallbackDataBuilder::new()
                                .content(avatar_url)
                                .flags(MessageFlags::EPHEMERAL)
                                .build();
                            hc.create_interaction_original(
                                ac.id,
                                &ac.token,
                                &InteractionResponse::ChannelMessageWithSource(res),
                            )
                            .exec()
                            .await?;
                        }
                        // dispatch to groupic
                        if ac.data.id == groupic_command.id.unwrap() {
                            let cov = ac
                                .data
                                .options
                                .into_iter()
                                .find(|cdo| cdo.name == "channel")
                                .unwrap()
                                .value;
                            let ci = match cov {
                                CommandOptionValue::Channel(ci) => ci,
                                _ => {
                                    error!(
                                        "Should get guild voice channel but instead got {:?}",
                                        cov
                                    );
                                    continue;
                                }
                            };
                            dbg_trace!(&ci);
                            let gi = match ac.guild_id {
                                Some(gi) => gi,
                                None => {
                                    error!("Command cannot be used outside of a guild");
                                    continue;
                                }
                            };
                            let c = hc.channel(ci).exec().await?.model().await?;
                            let gc = match c {
                                Channel::Guild(gc) => gc,
                                _ => {
                                    error!(
                                        "Should get guild voice channel but instead got {:?}",
                                        c
                                    );
                                    continue;
                                }
                            };
                            let vc = match gc {
                                GuildChannel::Voice(vc) => vc,
                                _ => {
                                    error!(
                                        "Should get guild voice channel but instead got {:?}",
                                        gc
                                    );
                                    continue;
                                }
                            };

                            dbg_trace!(&gi);
                            let voice_states = match cache.voice_channel_states(ci) {
                                Some(vcss) => vcss,
                                None => {
                                    error!("Failed to get voice states for channel {}", vc.name);
                                    continue;
                                }
                            };
                            let mut v_m: Vec<_> = Vec::with_capacity(1 << 5); // 128
                            for vs in voice_states.inspect(|vs| {
                                dbg_trace!(vs.user_id);
                            }) {
                                if vs.channel_id.unwrap() == ci {
                                    match vs.member.clone() {
                                        Some(m) => {
                                            v_m.push(m);
                                        }
                                        None => {
                                            let m: Member = hc
                                                .guild_member(gi, vs.user_id)
                                                .exec()
                                                .await?
                                                .model()
                                                .await?;
                                            v_m.push(m);
                                        }
                                    }
                                }
                            }
                            dbg_trace!(&v_m);
                            let v_a: Vec<_> = v_m
                                .iter()
                                .map(|m| match m.avatar.as_ref() {
                                    Some(s) => cdn::get_guild_member_avatar(
                                        gi,
                                        m.user.id,
                                        s,
                                        cdn::PJWG::PNG,
                                    ),
                                    None => match m.user.avatar.as_ref() {
                                        Some(s) => {
                                            cdn::get_user_avatar(m.user.id, s, cdn::PJWG::PNG)
                                        }
                                        None => cdn::get_default_user_avatar(m.user.discriminator),
                                    },
                                })
                                .collect();

                            // download avatars to this temp dir
                            let avatars_dir = TempDir::new("avatars").unwrap();
                            // construct async download tasks for each image file
                            let rc = reqwest::Client::default();
                            let download_futs: Vec<_> = v_a
                                .into_iter()
                                .map(|url| async {
                                    let mut file = {
                                        let url = reqwest::Url::from_str(&url).unwrap();
                                        let fname = url.path_segments().unwrap().last().unwrap();
                                        fs::File::create(avatars_dir.path().join(fname))
                                            .await
                                            .unwrap()
                                    };
                                    let res = rc.get(url).send().await.unwrap();
                                    let img = res.bytes().await.unwrap();
                                    file.write_all(img.as_ref()).await
                                })
                                .collect();
                            // run downloads concurrently
                            try_join_all(download_futs).await?;

                            let groupic_path = avatars_dir.path().join("groupic.png");

                            let ad_clone = avatars_dir.path().to_owned();
                            let vn_clone = vc.name.clone();
                            let gp_clone = groupic_path.clone();
                            dbg_debug!(&avatars_dir.path().is_dir());
                            spawn_blocking(|| {
                                gen_pic::generate_group_pic(ad_clone, gp_clone, 5, vn_clone);
                            })
                            .await?;
                            dbg_debug!(&groupic_path.is_file());

                            // let content = vc.name
                            //     + "\n"
                            //     + &v_m
                            //         .into_iter()
                            //         .map(|m| m.nick.unwrap_or(m.user.name))
                            //         .collect::<Vec<_>>()
                            //         .join("\n");
                            let content = "Oats curry everyone!".to_owned();
                            let cbd = twilight_util::builder::CallbackDataBuilder::new()
                                .content(content)
                                // .flags(MessageFlags::EPHEMERAL)
                                .build();
                            hc.create_interaction_original(
                                ac.id,
                                &ac.token,
                                &InteractionResponse::ChannelMessageWithSource(cbd),
                            )
                            .exec()
                            .await?;
                            let groupic_bytes = fs::read(groupic_path)
                                .await
                                .with_context(|| "Failed to read groupic.png")?;
                            hc.update_interaction_original(&ac.token)?
                                .files(&[("groupic.png", &groupic_bytes)])
                                .exec()
                                .await?;
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    Ok(())
}

