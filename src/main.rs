use serenity::client::Client;
use serenity::prelude::*;
use serenity::model::prelude::*;
use serenity::framework::standard::{StandardFramework};
use std::env;
use std::sync::Arc;
use std::time::Duration;

mod store;
mod commands;
mod api;

use store::{ Config, ConfigKey };
use commands::{GENERAL_GROUP, ROLES_GROUP, MY_HELP};

struct Handler;

impl EventHandler for Handler {
	fn ready(&self, ctx: Context, _data_about_bot: Ready) {
		ctx.set_activity(Activity::playing("~help"));
	}
}

fn main() {
	println!("Token?");
	// Login with a bot token from the environment
	let mut client = Client::new(&env::var("DISCORD_TOKEN").expect("token"), Handler)
		.expect("Error creating client");
	println!("Token!");

	{
		let mut data = client.data.write();

		let folder = ::std::env::current_dir().unwrap().join("data");

		if !folder.exists() {
			::std::fs::create_dir(&folder).expect("Should be able to create a folder in cwd");
		}

		let path = folder.join("config.json");

		let config = Config::load_or_create(path);

		data.insert::<ConfigKey>(Arc::new(RwLock::new(config)));
	}

	let prefix = env::var("BOT_PREFIX").unwrap_or("!".to_string());

	client.with_framework(StandardFramework::new()
		.configure(|c| c.prefix(&prefix))
		.group(&GENERAL_GROUP)
		.group(&ROLES_GROUP)
		.help(&MY_HELP)
		.on_dispatch_error(|context, msg, error| {
			use serenity::framework::standard::DispatchError::*;

			match error {
				NotEnoughArguments { min, given } => {
					let s = format!("Need {} arguments, but only got {}.", min, given);

					let _ = msg.reply(&context, &s);
				}
				TooManyArguments { max, given } => {
					let s = format!("Max arguments allowed is {}, but got {}.", max, given);

					let _ = msg.reply(&context, &s);
				}
				_ => println!("Unhandled dispatch error."),
			}
		})
		.after(|ctx, msg, cmd_name, res| {
			println!("Processed: {}", cmd_name);
			if let Err(why) = res {
				if why.0.starts_with("#reject#") {
					why.0.split("#-#").skip(1).next().and_then(|r| r.parse::<u64>().ok()).and_then(|id| {
						::std::thread::sleep(Duration::from_secs(4));
						ctx.http.delete_message(*msg.channel_id.as_u64(), id).ok()
					});
				} else {
					println!("Error in {}: {:?}", cmd_name, why);
				}
			}
		}));

	// start listening for events by starting a single shard
	if let Err(why) = client.start() {
		println!("An error occurred while running the client: {:?}", why);
	}
}
