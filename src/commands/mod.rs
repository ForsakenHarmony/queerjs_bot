use serenity::{
	framework::standard::{
		help_commands,
		macros::help,
		Args,
		CommandGroup,
		CommandResult,
		HelpOptions,
	},
	model::prelude::*,
	prelude::*,
};
use std::collections::HashSet;

pub mod general;
pub mod owner;
pub mod roles;

#[help]
#[command_not_found_text = "could not find command `{}`."]
#[max_levenshtein_distance(3)]
#[lacking_permissions = "Hide"]
pub(crate) async fn my_help(
	context: &Context,
	msg: &Message,
	args: Args,
	help_options: &'static HelpOptions,
	groups: &[&'static CommandGroup],
	owners: HashSet<UserId>,
) -> CommandResult {
	let _ = help_commands::with_embeds(context, msg, args, &help_options, groups, owners).await;
	Ok(())
}
