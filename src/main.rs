use std::collections::HashMap;
use std::sync::Arc;
use serenity::async_trait;
use serenity::Client;
use serenity::framework::StandardFramework;
use serenity::model::prelude::*;
use serenity::model::prelude::application_command::*;
use serenity::prelude::*;
use tokio::sync::RwLock;
use serenity::model::webhook::Webhook;

#[derive(Debug)]
struct GroupData {
    pub title: String,
    pub description: String,
    pub author: UserId,
    pub channels: Vec<(ChannelId, WebhookId)>
}

// currently all the data are stored in the memory, couldn't bother making it persistent lol
struct Bot {
    db_channel_groups: Arc<RwLock<HashMap<ChannelId, String>>>,
    db_groups: Arc<RwLock<HashMap<String, GroupData>>>,
}

const LINK_COMMAND: &str = "link";
const NEW_COMMAND: &str = "new";
const LIST_COMMAND: &str = "list";

#[async_trait]
impl EventHandler for Bot {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot { return; }

        if let Some(group_key) = self.db_channel_groups.read().await.get(&msg.channel_id) {
            if let Some(group) = self.db_groups.read().await.get(group_key) {
                // retrieve all the webhooks
                let webhooks = group.channels
                    .iter()
                    .filter(|(channel, _)| channel != &msg.channel_id)
                    .map(|(_, webhook)| webhook.to_webhook(&ctx))
                    .collect::<Vec<_>>();

                let webhooks = futures::future::join_all(webhooks).await;

                let mut index = 0_u16;

                futures::future::join_all(
                    group.channels
                        .iter()
                        .filter_map(|(channel, webhook_id)| {
                            // ignore the current channel
                            if channel == &msg.channel_id { return None; }

                            let webhook = webhooks.get(index as usize).unwrap();

                            // check if this webhook failed to be retrieved
                            if webhook.is_err() {
                                println!("[!] Failed to retrieve webhook of channel id: {}, webhook id: {}", channel.0, webhook_id.0);
                                return None;
                            }

                            let webhook = webhook.as_ref().unwrap();

                            index += 1;

                            // dispatch the webhook async process
                            Some(Webhook::execute(
                                webhook,
                                &ctx,
                                false,
                                |x|
                                    x.content(&msg.content)
                                        .username(&msg.author.tag())
                                        .avatar_url(&msg.author.face())
                            ))
                        })
                        .collect::<Vec<_>>()
                ).await;
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("Bot {} is ready", ready.user.name);

        // GuildId(889344940260327454)
        //     .set_application_commands(&ctx, |commands| {
        //         commands.create_application_command(|command|
        //             command
        //                 .name(NEW_COMMAND)
        //                 .description("Create a new link to link between channels from every servers (that has this bot joined)")
        //                 .create_option(|option|
        //                     option
        //                         .name("link_id")
        //                         .description("The link id (can be [a-z0-9-_])")
        //                         .kind(ApplicationCommandOptionType::String)
        //                         .required(true)
        //                 )
        //                 .create_option(|option|
        //                     option
        //                         .name("title")
        //                         .description("The title of this link")
        //                         .kind(ApplicationCommandOptionType::String)
        //                         .required(true)
        //                 )
        //                 .create_option(|option|
        //                     option
        //                         .name("description")
        //                         .description("The description of this link")
        //                         .kind(ApplicationCommandOptionType::String)
        //                         .required(true)
        //                 )
        //             )
        //             .create_application_command(|command|
        //                 command
        //                     .name(LINK_COMMAND)
        //                     .description("Links a channel to a link")
        //                     .create_option(|option| {
        //                         option
        //                             .name("link_id")
        //                             .description("The link id")
        //                             .kind(ApplicationCommandOptionType::String)
        //                             .required(true)
        //                     })
        //                     .create_option(|option| {
        //                         option
        //                             .name("channel")
        //                             .description("The channel you wanted to link")
        //                             .kind(ApplicationCommandOptionType::Channel)
        //                             .required(true)
        //                     })
        //             )
        //             .create_application_command(|command|
        //                 command
        //                     .name(LIST_COMMAND)
        //                     .description("Lists all the public and available links")
        //             )
        //     }).await.unwrap();

        // initialize slash commands
        ApplicationCommand::create_global_application_command(&ctx, |command| {
            command
                .name(NEW_COMMAND)
                .description("Create a new link to link between channels from every servers (that has this bot joined)")
                .create_option(|option|
                    option
                        .name("link_id")
                        .description("The link id (can be [a-z0-9-_])")
                        .kind(ApplicationCommandOptionType::String)
                        .required(true)
                )
                .create_option(|option|
                    option
                        .name("title")
                        .description("The title of this link")
                        .kind(ApplicationCommandOptionType::String)
                        .required(true)
                )
                .create_option(|option|
                    option
                        .name("description")
                        .description("The description of this link")
                        .kind(ApplicationCommandOptionType::String)
                        .required(true)
                )
        }).await.expect("Failed to create the new command");

        ApplicationCommand::create_global_application_command(&ctx, |command| {
            command
                .name(LINK_COMMAND)
                .description("Links a channel to a link")
                .create_option(|option| {
                    option
                        .name("link_id")
                        .description("The link id")
                        .kind(ApplicationCommandOptionType::String)
                        .required(true)
                })
                .create_option(|option| {
                    option
                        .name("channel")
                        .description("The channel you wanted to link")
                        .kind(ApplicationCommandOptionType::Channel)
                        .required(true)
                })
        }).await.expect("Failed to create the link command");

        ApplicationCommand::create_global_application_command(&ctx, |command|
            command
                .name(LIST_COMMAND)
                .description("Lists all public and available links")
        ).await.expect("Failed to create the list command");
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            // deny interactions from DMs
            if command.guild_id.is_none() {
                command
                    .create_interaction_response(&ctx, |resp| {
                        resp.kind(InteractionResponseType::ChannelMessageWithSource)
                            .interaction_response_data(|msg|
                                msg.content("This can only be called on a server")
                            )
                    }).await.expect("Failed to respond to interaction command failure: \"server only\"");

                return;
            }

            match command.data.name.as_str() {
                LINK_COMMAND => {
                    // check if the user has the permission to manage channels
                    if !command
                        .member.as_ref().unwrap()
                        .permissions.unwrap()
                        .manage_channels() {
                        command.create_interaction_response(&ctx, |resp|
                            resp.kind(InteractionResponseType::ChannelMessageWithSource)
                                .interaction_response_data(|mesg|
                                    mesg.content("Insufficient permission, this command requires the manage channel permission")
                                        .flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL)
                                )
                        ).await.expect("Failed to respond to interaction command failure `link`: \"insufficient permission\"");

                        return;
                    }

                    // get params
                    let link_id =
                        command.data.options
                            .iter()
                            .find(|i| i.name == "link_id").unwrap()
                            .value.as_ref().unwrap()
                            .as_str().unwrap()
                            .to_string();

                    let channel_id =
                        ChannelId(
                            command.data.options
                                .iter()
                                .find(|i| i.name == "channel").unwrap()
                                .value.as_ref().unwrap()
                                .as_str().unwrap()
                                .parse::<u64>().expect("Failed to parse discord's channel id into u64")
                        );

                    // check if the link exists
                    {
                        if !&self.db_groups.read().await.contains_key(&link_id) {
                            command.create_interaction_response(&ctx, |resp|
                                resp.kind(InteractionResponseType::ChannelMessageWithSource)
                                    .interaction_response_data(|mes|
                                        mes.content(format!("Link {} doesn't exist!", link_id))
                                            .flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL)
                                    )
                            ).await.expect("Failed to respond to interaction command failure: \"link doesn't exist\"");

                            return;
                        }
                    }

                    // it exists, now we create a webhook for this channel and link it
                    let channel_webhook = channel_id.to_channel(&ctx).await.unwrap()
                        .guild().unwrap()
                        .create_webhook(&ctx, "Chat linker")
                        .await.expect("Failed to create webhook");

                    self.db_channel_groups.write().await
                        .insert(channel_id.clone(), link_id.clone());

                    self.db_groups.write().await
                        .get_mut(&link_id).unwrap()
                        .channels.push((channel_id.clone(), channel_webhook.id));

                    command.create_interaction_response(&ctx, |resp|
                        resp.interaction_response_data(|msg|
                            msg.content(format!("Successfully linked channel {} to `{}`", channel_id.mention(), link_id))
                                .flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL)
                        )).await.expect("Failed to respond to interaction command `link` success");
                }

                NEW_COMMAND => {
                    let link_id =
                        command.data.options
                            .iter()
                            .find(|i| i.name == "link_id").unwrap()
                            .value.as_ref().unwrap()
                            .as_str().unwrap()
                            .to_string();

                    let link_title =
                        command.data.options
                            .iter()
                            .find(|i| i.name == "title").unwrap()
                            .value.as_ref().unwrap()
                            .as_str().unwrap()
                            .to_string();

                    let link_description =
                        command.data.options
                            .iter()
                            .find(|i| i.name == "description").unwrap()
                            .value.as_ref().unwrap()
                            .as_str().unwrap()
                            .to_string();

                    // check if the link name is already used
                    {
                        if self.db_groups.read().await.contains_key(link_id.as_str()) {
                            // name already used
                            command.create_interaction_response(&ctx, |resp|
                                resp.kind(InteractionResponseType::ChannelMessageWithSource)
                                    .interaction_response_data(|mes|
                                        mes.content(format!("Link {} is already used, please choose another name", link_id))
                                            .flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL)
                                    )
                            ).await.expect("Failed to respond to the command `new`: \"name already used\"");

                            return;
                        }
                    }

                    // create the link
                    self.db_groups
                        .write().await
                        .insert(link_id.clone(), GroupData {
                            title: link_title,
                            description: link_description,
                            author: command.user.id,
                            channels: vec![]
                        });

                    // success
                    command.create_interaction_response(&ctx, |resp| {
                        resp.kind(InteractionResponseType::ChannelMessageWithSource)
                            .interaction_response_data(|mes| {
                                mes.content(format!("Link `{}` successfully created", link_id))
                                    .flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL)
                            })
                    }).await.expect("Failed to respond the `new` command success");
                }

                LIST_COMMAND => {
                    // just list everything ig
                    let db_groups_read = &self.db_groups.read().await;

                    command.create_interaction_response(&ctx, |resp|
                        resp.kind(InteractionResponseType::ChannelMessageWithSource)
                            .interaction_response_data(|mes|
                                mes.create_embed(|e|
                                    e.title("Public links")
                                        .fields(
                                            db_groups_read
                                                .iter()
                                                .map(|(id, data)| {
                                                    (format!("`{}` {}", id, data.title),
                                                     format!("```{}```{} channels linked\nBy {}", data.description, data.channels.len(), data.author.mention()),
                                                     false)
                                                })
                                        )
                                )
                            )
                    ).await.expect("Failed to respond to the list command interaction");
                }

                _ => {
                    command
                        .create_interaction_response(&ctx, |resp| {
                            resp.kind(InteractionResponseType::ChannelMessageWithSource)
                                .interaction_response_data(|mesg|
                                    mesg.content("Unknown interaction")
                                        .flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL)
                                )
                        }).await.expect("Failed to respond to an unknown interaction");
                }
            };
        }
    }
}

macro_rules! hashmap {
    ($( $key: expr => $val: expr ),*) => {{
         let mut map = ::std::collections::HashMap::new();
         $( map.insert($key, $val); )*
         map
    }}
}

#[tokio::main]
async fn main() {
    // Login with a bot token from the environment
    let mut client = Client::builder(std::env::var("token").expect("A bot token"))
        .event_handler(Bot {
            db_channel_groups: Arc::new(RwLock::new(hashmap![
                // nothing to see here, just some testing data
                // ChannelId(915230411259523103) => "test".to_string(),
                // ChannelId(915230429114683403) => "test".to_string()
            ])),
            db_groups: Arc::new(RwLock::new(hashmap![
                // "test".to_string() => GroupData {
                //     title: "Hello World".to_string(),
                //     description: "Very cool amirite".to_string(),
                //     author: UserId(574128504451366913),
                //     channels: vec![
                //         (ChannelId(915230411259523103), WebhookId(915231896097677362)),
                //         (ChannelId(915230429114683403), WebhookId(915232747285528657))
                //     ]
                // }
            ]))
        })
        .application_id(915192869139148860)
        .framework(StandardFramework::new())
        .await
        .expect("Error creating client");

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }
}