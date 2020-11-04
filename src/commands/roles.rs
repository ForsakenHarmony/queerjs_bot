use crate::api::Api;
use serenity::{
	framework::standard::{macros::command, Args, CommandResult},
	model::prelude::*,
	prelude::*,
};

async fn send_ok(ctx: &Context, msg: &Message) -> CommandResult {
	msg.react(ctx, ReactionType::Unicode("✅".to_string()))
		.await?;
	Ok(())
}

#[command]
#[description = "Lists roles you're allowed to assign to yourself"]
#[only_in(guilds)]
async fn list(ctx: &Context, msg: &Message) -> CommandResult {
	Api::new(ctx, msg).list_roles().await
}

#[command]
#[usage = "{role name}"]
#[num_args(1)]
#[description = "Adds a role to you if you're allowed to have it"]
#[only_in(guilds)]
async fn add(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
	let role: String = args.single()?;
	Api::new(ctx, msg).add_role(role).await?;
	send_ok(ctx, msg).await
}

#[command]
#[usage = "{role name}"]
#[num_args(1)]
#[description = "Removes one of your roles"]
#[only_in(guilds)]
async fn remove(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
	let role: String = args.single()?;
	Api::new(ctx, msg).remove_role(role).await?;
	send_ok(ctx, msg).await
}

#[command]
#[usage = "{role name}"]
#[num_args(1)]
#[description = "Create a role with the given name"]
#[only_in(guilds)]
#[required_permissions(MANAGE_ROLES)]
async fn create_role(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
	let role: String = args.single()?;
	Api::new(ctx, msg).create_role(role).await?;
	send_ok(ctx, msg).await
}

#[command]
#[usage = "{role name} {alias name}"]
#[num_args(2)]
#[description = "Add an alias for a role"]
#[only_in(guilds)]
#[required_permissions(MANAGE_ROLES)]
async fn alias_role(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
	let role: String = args.single()?;
	let alias: String = args.single()?;
	Api::new(ctx, msg).alias_role(role, alias).await?;
	send_ok(ctx, msg).await
}

#[command]
#[usage = "{alias name}"]
#[num_args(1)]
#[description = "Remove an alias for a role"]
#[only_in(guilds)]
#[required_permissions(MANAGE_ROLES)]
async fn remove_alias(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
	let alias: String = args.single()?;
	Api::new(ctx, msg).remove_alias(alias).await?;
	send_ok(ctx, msg).await
}

#[command]
#[usage = "{role name}"]
#[num_args(1)]
#[description = "Allow users to assign a role to themselves"]
#[only_in(guilds)]
#[required_permissions(MANAGE_ROLES)]
async fn allow_role(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
	let role: String = args.single()?;
	Api::new(ctx, msg).allow_role(role).await?;
	send_ok(ctx, msg).await
}

#[command]
#[usage = "{role name}"]
#[num_args(1)]
#[description = "Deny users to assign a role to themselves (if it was previously allowed)"]
#[only_in(guilds)]
#[required_permissions(MANAGE_ROLES)]
async fn deny_role(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
	let role: String = args.single()?;
	Api::new(ctx, msg).deny_role(role).await?;
	send_ok(ctx, msg).await
}
