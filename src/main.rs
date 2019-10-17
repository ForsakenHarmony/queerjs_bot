use serenity::client::Client;
use serenity::prelude::*;
use serenity::model::prelude::*;
use serenity::framework::standard::{StandardFramework, CommandResult, help_commands, macros::{
	command,
	group,
	help,
}, Args, HelpOptions, CommandGroup};
use std::env;
use std::collections::{HashSet, HashMap};
use serde_derive::{Serialize, Deserialize};
use serde_json::{from_reader, to_writer_pretty};
use std::io::{BufReader, BufWriter};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

group!({
    name: "general",
    options: {},
    commands: [ping],
});

group!({
    name: "roles",
    options: {},
    commands: [list, add, remove, alias_role, create_role, allow_role, deny_role],
});

#[derive(Serialize, Deserialize)]
struct InnerConfig {
	allowed_roles: Vec<u64>,
	aliases: HashMap<String, u64>,
}

struct Config {
	inner_config: InnerConfig,
	path: PathBuf,
}

struct ConfigKey;

impl TypeMapKey for ConfigKey {
	type Value = Arc<RwLock<Config>>;
}

impl Config {
	fn get_roles(&self) -> &Vec<u64> {
		&self.inner_config.allowed_roles
	}

	fn add_role(&mut self, role: u64) {
		if self.inner_config.allowed_roles.contains(&role) { return; }

		self.inner_config.allowed_roles.push(role);

		self.save();
	}

	fn remove_role(&mut self, role: u64) {
		if !self.inner_config.allowed_roles.contains(&role) { return; }

		self.inner_config.allowed_roles.remove(self.inner_config.allowed_roles.iter().position(|r| &role == r).unwrap());
		let to_remove = self.inner_config.aliases.iter().filter_map(|(a, r)| if *r == role { Some(a.clone()) } else { None }).collect::<Vec<_>>();
		to_remove.iter().for_each(|a| {
			self.inner_config.aliases.remove(a);
		});

		self.save();
	}

	fn add_alias(&mut self, role: u64, alias: String) {
		if !self.inner_config.allowed_roles.contains(&role) { return; }

		self.inner_config.aliases.insert(alias, role);

		self.save();
	}

	fn remove_alias(&mut self, alias: String) {
		self.inner_config.aliases.remove(&alias);

		self.save();
	}

	fn get_aliases(&self) -> &HashMap<String, u64> {
		&self.inner_config.aliases
	}

	fn resolve_alias(&self, alias: &String) -> Option<u64> {
		self.inner_config.aliases.get(alias).map(|e| *e)
	}

	fn create<P: AsRef<Path>>(path: P) -> Config {
		let cfg = Config {
			inner_config: InnerConfig {
				allowed_roles: Vec::new(),
				aliases: HashMap::new(),
			},
			path: PathBuf::from(path.as_ref()),
		};

		cfg.save();

		cfg
	}

	fn load_or_create<P: AsRef<Path>>(path: P) -> Config {
		let file = match File::open(path.as_ref()) {
			Ok(f) => f,
			_ => return Self::create(path.as_ref()),
		};

		let reader = BufReader::new(file);
		let inner: InnerConfig = match from_reader(reader) {
			Ok(i) => i,
			_ => return Self::create(path.as_ref()),
		};

		Config {
			inner_config: inner,
			path: PathBuf::from(path.as_ref()),
		}
	}

	fn save(&self) {
		let file = File::create(&self.path).expect("should be able to open/create the file");
		let writer = BufWriter::new(file);

		to_writer_pretty(writer, &self.inner_config).expect("should be able to write to the file");
	}
}

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

	client.with_framework(StandardFramework::new()
		.configure(|c| c.prefix("~")) // set the bot's prefix to "~"
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

#[command]
fn ping(ctx: &mut Context, msg: &Message) -> CommandResult {
	msg.reply(ctx, "Pong!")?;

	Ok(())
}


fn get_cfg(ctx: &mut Context) -> Arc<RwLock<Config>> {
	let data = ctx.data.write();
	let cfg = data.get::<ConfigKey>().expect("Should have Config").clone();
	cfg
}

fn send_ok(ctx: &mut Context, msg: &Message) -> CommandResult {
	println!("ok?");
	msg.react(&ctx, ReactionType::Unicode("✅".to_string()))?;
	println!("ok!");
	Ok(())
}

fn send_reject(ctx: &mut Context, msg: &Message) -> CommandResult {
	msg.react(ctx, ReactionType::Unicode("❌".to_string()))?;
	Err("#reject#".into())
}

fn send_reject_with_msg(ctx: &mut Context, msg: &Message, reason: impl AsRef<str>) -> CommandResult {
	msg.react(&ctx, ReactionType::Unicode("❌".to_string()))?;
	let response = msg.reply(&ctx, reason)?;
	Err(format!("#reject#-#{}", response.id.as_u64()).into())
}

fn lookup_role<'a>(ctx: &mut Context, guild: &'a Guild, role: String) -> Option<&'a Role> {
	let cfg = get_cfg(ctx);
	let alias = cfg.read().resolve_alias(&role);

	let role = match (alias.and_then(|a| guild.roles.get(&RoleId(a))), guild.role_by_name(&role)) {
		(Some(r), None) => r,
		(None, Some(r)) => r,
		_ => return None,
	};

	Some(role)
}

#[command]
#[description = "Lists roles you're allowed to assign to yourself"]
#[only_in(guilds)]
fn list(ctx: &mut Context, msg: &Message) -> CommandResult {
	let guild = match msg.guild(&ctx) {
		Some(g) => g,
		_ => return send_reject_with_msg(ctx, msg, "Not in a guild?"),
	};
	let guild = guild.read();

	let cfg_guard = get_cfg(ctx);
	let cfg = cfg_guard.read();

	let roles = cfg.get_roles().iter().filter_map(|id| {
		let aliases = cfg.get_aliases().iter().filter_map(|(a, r)| if r == id { Some(format!("`{}`", a.clone())) } else { None }).collect::<Vec<_>>();
		guild.roles.get(&RoleId(*id)).map(|r| {
			let main = format!("`{}`", r.name.clone());
			if aliases.len() > 0 {
				format!("{} - aliases: {}", main, aliases.join(","))
			} else {
				main
			}
		})
	}).collect::<Vec<_>>();

	msg.channel_id.send_message(&ctx.http, |m| {
		m.embed(|e| {
			e.title("Available Roles");
			e.description(roles.join("\n"));
			e
		});
		m
	})?;

	Ok(())
}

#[command]
#[usage = "{role name}"]
#[num_args(1)]
#[description = "Adds a role to you if you're allowed to have it"]
#[only_in(guilds)]
fn add(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
	{
		let role: String = args.single()?;

		let guild = match msg.guild(&ctx) {
			Some(g) => g,
			_ => return send_reject_with_msg(ctx, msg, "Not in a guild?"),
		};
		let guild = guild.read();

		let role = match lookup_role(ctx, &guild, role) {
			Some(r) => r,
			_ => return send_reject_with_msg(ctx, msg, "Role not found"),
		};

		let cfg_guard = get_cfg(ctx);
		let cfg = cfg_guard.read();
		let roles = cfg.get_roles();
		if !roles.contains(&role.id.as_u64()) { return send_reject_with_msg(ctx, msg, "Not an allowed role"); }

		let mut member = msg.member(&ctx).expect("We already checked this");

		if let Some(_) = member.roles(&ctx).expect("Users always have roles").iter().find(|p| p.id == role.id) {
			return send_reject_with_msg(ctx, msg, "You already have this role");
		}

		match member.add_role(&ctx, role.id) {
			Ok(_) => {}
			_ => return send_reject_with_msg(ctx, msg, "Couldn't set role")
		}
	}

	send_ok(ctx, msg)
}

#[command]
#[usage = "{role name}"]
#[num_args(1)]
#[description = "Removes one of your roles"]
#[only_in(guilds)]
fn remove(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
	{
		let role: String = args.single()?;

		let guild = match msg.guild(&ctx) {
			Some(g) => g,
			_ => return send_reject_with_msg(ctx, msg, "Not in a guild?"),
		};
		let guild = guild.read();

		let role = match lookup_role(ctx, &guild, role) {
			Some(r) => r,
			_ => return send_reject_with_msg(ctx, msg, "Role not found"),
		};

		let cfg_guard = get_cfg(ctx);
		let cfg = cfg_guard.read();
		let roles = cfg.get_roles();
		if !roles.contains(&role.id.as_u64()) { return send_reject_with_msg(ctx, msg, "Not an allowed role"); }

		let mut member = msg.member(&ctx).expect("We already checked this");

		if let None = member.roles(&ctx).expect("Users always have roles").iter().find(|p| p.id == role.id) {
			return send_reject_with_msg(ctx, msg, "You don't have this role");
		}

		match member.remove_role(&ctx, role.id) {
			Ok(_) => {}
			_ => return send_reject_with_msg(ctx, msg, "Couldn't remove role")
		}
	}

	send_ok(ctx, msg)
}

#[command]
#[usage = "{role name}"]
#[num_args(1)]
#[description = "Create a role with the given name"]
#[only_in(guilds)]
#[required_permissions(MANAGE_ROLES)]
fn create_role(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
	let role: String = args.single()?;

	{
		let guild = match msg.guild(&ctx) {
			Some(g) => g,
			_ => return send_reject_with_msg(ctx, msg, "Not in a guild?"),
		};
		let guild = guild.read();

		if let Some(_) = guild.role_by_name(&role) {
			return send_reject_with_msg(ctx, msg, "A role with that name already exists");
		}

		let role = guild.create_role(&ctx, |r| {
			r.name(role);
			r
		})?;

		let cfg = get_cfg(ctx);
		cfg.write().add_role(*role.id.as_u64());
	}

	send_ok(ctx, msg)
}

#[command]
#[usage = "{role name} {alias name}"]
#[num_args(2)]
#[description = "Add an alias for a role"]
#[only_in(guilds)]
#[required_permissions(MANAGE_ROLES)]
fn alias_role(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
	let role: String = args.single()?;
	let alias: String = args.single()?;

	let guild = match msg.guild(&ctx) {
		Some(g) => g,
		_ => return send_reject_with_msg(ctx, msg, "Not in a guild?"),
	};
	let guild = guild.read();

	let role = match guild.role_by_name(&role) {
		Some(r) => r,
		_ => return send_reject_with_msg(ctx, msg, "Role not found"),
	};

	{
		let cfg_guard = get_cfg(ctx);
		let cfg = cfg_guard.read();
		let roles = cfg.get_roles();
		if !roles.contains(&role.id.as_u64()) { return send_reject_with_msg(ctx, msg, "Not an allowed role (aliases are only possible for allowed roles)"); }
	}

	{
		println!("wew");

		let cfg = get_cfg(ctx);
		println!("myb?");
		cfg.write().add_alias(*role.id.as_u64(), alias);

		println!("no?");
	}

	send_ok(ctx, msg)
}

#[command]
#[usage = "{role name}"]
#[num_args(1)]
#[description = "Allow users to assign a role to themselves"]
#[only_in(guilds)]
#[required_permissions(MANAGE_ROLES)]
fn allow_role(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
	let role: String = args.single()?;
	{
		let guild = match msg.guild(&ctx) {
			Some(g) => g,
			_ => return send_reject_with_msg(ctx, msg, "Not in a guild?"),
		};
		let guild = guild.read();

		let role = match guild.role_by_name(&role) {
			Some(r) => r,
			_ => return send_reject_with_msg(ctx, msg, "Role not found"),
		};

		let cfg = get_cfg(ctx);
		cfg.write().add_role(*role.id.as_u64());
	}

	send_ok(ctx, msg)
}

#[command]
#[usage = "{role name}"]
#[num_args(1)]
#[description = "Deny users to assign a role to themselves (if it was previously allowed)"]
#[only_in(guilds)]
#[required_permissions(MANAGE_ROLES)]
fn deny_role(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
	let role: String = args.single()?;

	let guild = match msg.guild(&ctx) {
		Some(g) => g,
		_ => return send_reject_with_msg(ctx, msg, "Not in a guild?"),
	};
	let guild = guild.read();

	let role = match guild.role_by_name(&role) {
		Some(r) => r,
		_ => return send_reject_with_msg(ctx, msg, "Role not found"),
	};

	{
		let cfg_guard = get_cfg(ctx);
		let cfg = cfg_guard.read();
		let roles = cfg.get_roles();
		if !roles.contains(&role.id.as_u64()) { return send_reject_with_msg(ctx, msg, "Not an allowed role"); }
	}

	{
		let cfg = get_cfg(ctx);
		cfg.write().remove_role(*role.id.as_u64());
	}

	send_ok(ctx, msg)
}

#[help]
fn my_help(
	context: &mut Context,
	msg: &Message,
	args: Args,
	help_options: &'static HelpOptions,
	groups: &[&'static CommandGroup],
	owners: HashSet<UserId>,
) -> CommandResult {
	help_commands::with_embeds(context, msg, args, help_options, groups, owners)
}
