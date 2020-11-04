use std::{collections::HashSet, env, sync::Arc};

use serenity::{
	async_trait,
	client::{bridge::gateway::ShardManager, Client},
	framework::standard::{
		macros::{group, hook},
		StandardFramework,
	},
	http::Http,
	model::{
		event::ResumedEvent,
		gateway::{Activity, Ready},
	},
	prelude::*,
};
use tracing::{debug, error, info, warn};
use tracing_subscriber::{EnvFilter, FmtSubscriber};

use store::{Config, ConfigKey};

mod api;
mod commands;
mod store;

use commands::{general::*, owner::*, roles::*, MY_HELP};
use serenity::{framework::standard::CommandResult, model::channel::Message};

struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
	type Value = Arc<Mutex<ShardManager>>;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
	async fn ready(&self, ctx: Context, ready: Ready) {
		info!("connected as {}", ready.user.name);

		let prefix = env::var("BOT_PREFIX").unwrap_or("!".to_string());
		ctx.set_activity(Activity::listening(&format!("{}help", prefix)))
			.await;
	}

	async fn resume(&self, _: Context, _: ResumedEvent) {
		info!("resumed");
	}
}

#[group("general")]
#[commands(ping, quit)]
pub struct General;

#[group("roles")]
#[only_in(guilds)]
#[commands(
	list,
	add,
	remove,
	alias_role,
	remove_alias,
	create_role,
	allow_role,
	deny_role
)]
pub struct Roles;

#[hook]
async fn after(_ctx: &Context, _msg: &Message, command_name: &str, command_result: CommandResult) {
	match command_result {
		Ok(()) => info!("processed command '{}'", command_name),
		Err(why) => {
			warn!("command '{}' returned error {:?}", command_name, why);
		}
	}
}

#[tokio::main]
async fn main() {
	dotenv::dotenv().expect("failed to load .env file");

	let sub = FmtSubscriber::builder()
		.pretty()
		// .compact()
		.with_env_filter(EnvFilter::from_default_env())
		.finish();

	tracing::subscriber::set_global_default(sub).expect("failed to start the logger.");

	let token = env::var("DISCORD_TOKEN")
		.expect("there should be a discord token in the DISCORD_TOKEN environment variable.");

	let prefix = env::var("BOT_PREFIX").unwrap_or("!".to_string());

	info!("found discord token, initializing client.");

	debug!("creating http client.");
	let http = Http::new_with_token(&token);

	debug!("fetching bot info.");
	// We will fetch your bot's owners and id
	let (owners, bot_id) = match http.get_current_application_info().await {
		Ok(info) => {
			let mut owners = HashSet::new();
			owners.insert(info.owner.id);
			if let Some(team) = info.team {
				owners.insert(team.owner_user_id);
			}

			(owners, info.id)
		}
		Err(why) => panic!("could not access application info: {:?}", why),
	};

	debug!("creating framework.");
	let framework = StandardFramework::new()
		.configure(|c| c.owners(owners).prefix(&prefix).on_mention(Some(bot_id)))
		.after(after)
		.group(&GENERAL_GROUP)
		.group(&ROLES_GROUP)
		.help(&MY_HELP);

	debug!("creating client.");
	let mut client = Client::builder(&token)
		.framework(framework)
		.event_handler(Handler)
		.await
		.expect("error creating client.");

	info!("client login successful.");

	{
		info!("opening data file (data/config.json).");
		let mut data = client.data.write().await;

		let folder = ::std::env::current_dir().unwrap().join("data");

		if !folder.exists() {
			::std::fs::create_dir(&folder).expect("Should be able to create a folder in cwd");
		}

		let path = folder.join("config.json");

		let config = Config::load_or_create(path);

		data.insert::<ConfigKey>(Arc::new(RwLock::new(config)));
		debug!("data loaded")
	}

	{
		let mut data = client.data.write().await;
		data.insert::<ShardManagerContainer>(client.shard_manager.clone());
	}

	let shard_manager = client.shard_manager.clone();

	tokio::spawn(async move {
		tokio::signal::ctrl_c()
			.await
			.expect("could not register ctrl+c handler");
		shard_manager.lock().await.shutdown_all().await;
	});

	info!("starting client");
	if let Err(why) = client.start().await {
		error!("client error: {:?}", why);
	}

	// client.with_framework(
	// 	StandardFramework::new()
	// 		.configure(|c| c.prefix(&prefix))
	// 		.group(&GENERAL_GROUP)
	// 		.group(&ROLES_GROUP)
	// 		.help(&MY_HELP)
	// 		.on_dispatch_error(|context, msg, error| {
	// 			use serenity::framework::standard::DispatchError::*;
	//
	// 			match error {
	// 				NotEnoughArguments { min, given } => {
	// 					let s = format!("Need {} arguments, but only got {}.", min, given);
	//
	// 					let _ = msg.reply(&context, &s);
	// 				}
	// 				TooManyArguments { max, given } => {
	// 					let s = format!("Max arguments allowed is {}, but got {}.", max, given);
	//
	// 					let _ = msg.reply(&context, &s);
	// 				}
	// 				_ => println!("Unhandled dispatch error."),
	// 			}
	// 		})
	// );

	// // start listening for events by starting a single shard
	// if let Err(why) = client.start() {
	// 	println!("An error occurred while running the client: {:?}", why);
	// }
}
