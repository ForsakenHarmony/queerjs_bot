use serenity::prelude::*;
use serenity::model::prelude::*;
use serenity::framework::standard::{
	CommandResult,
	Args,
	HelpOptions,
	CommandGroup,
	help_commands,
	macros::{
		command,
		group,
		help,
	}
};
use std::collections::HashSet;
use crate::api::Api;

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

fn send_ok(ctx: &mut Context, msg: &Message) -> CommandResult {
	msg.react(&ctx, ReactionType::Unicode("✅".to_string()))?;
	Ok(())
}

#[command]
fn ping(ctx: &mut Context, msg: &Message) -> CommandResult {
	msg.reply(ctx, "Pong!")?;

	Ok(())
}

#[command]
#[description = "Lists roles you're allowed to assign to yourself"]
#[only_in(guilds)]
fn list(ctx: &mut Context, msg: &Message) -> CommandResult {
	Api::new(ctx, msg).list_roles()
}

#[command]
#[usage = "{role name}"]
#[num_args(1)]
#[description = "Adds a role to you if you're allowed to have it"]
#[only_in(guilds)]
fn add(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
	let role: String = args.single()?;
	Api::new(ctx, msg).add_role(role)?;
	send_ok(ctx, msg)
}

#[command]
#[usage = "{role name}"]
#[num_args(1)]
#[description = "Removes one of your roles"]
#[only_in(guilds)]
fn remove(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
	let role: String = args.single()?;
	Api::new(ctx, msg).remove_role(role)?;
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
	Api::new(ctx, msg).create_role(role)?;
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
	Api::new(ctx, msg).alias_role(role, alias)?;
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
	Api::new(ctx, msg).allow_role(role)?;
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
	Api::new(ctx, msg).deny_role(role)?;
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
