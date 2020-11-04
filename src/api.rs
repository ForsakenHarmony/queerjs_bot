use crate::store::{Config, ConfigKey};
use serenity::{
	framework::standard::CommandResult,
	model::prelude::{Guild, Message, ReactionType, Role, RoleId},
	prelude::*,
};
use std::{fmt::Display, time::Duration};
use tracing::{debug, error, instrument};

pub struct Api<'a> {
	ctx: &'a Context,
	msg: &'a Message,
}

impl<'a> Api<'a> {
	pub fn new(ctx: &'a Context, msg: &'a Message) -> Self {
		Api { ctx, msg }
	}
}

impl<'a> Api<'a> {
	async fn with_cfg<T>(&self, f: impl Fn(&Config) -> T) -> T {
		let data = self.ctx.data.read().await;
		let cfg = data
			.get::<ConfigKey>()
			.expect("context should have the config");
		let cfg = cfg.read().await;
		f(&*cfg)
	}

	async fn with_cfg_mut<T>(&self, f: impl FnOnce(&mut Config) -> T) -> T {
		let data = self.ctx.data.write().await;
		let cfg = data
			.get::<ConfigKey>()
			.expect("context should have the config");
		let mut cfg = cfg.write().await;
		f(&mut *cfg)
	}

	async fn send_reject_with_msg<T>(&self, reason: impl Display + Clone) -> CommandResult<T> {
		self.msg
			.react(&self.ctx, ReactionType::Unicode("❌".to_string()))
			.await?;
		let response = self.msg.reply(&self.ctx, reason.clone()).await?;
		let http = self.ctx.http.clone();
		tokio::spawn(async move {
			tokio::time::delay_for(Duration::from_secs(5)).await;
			if let Err(why) = response.delete(http).await {
				error!("failed to delete rejection message: {:?}", why);
			}
		});
		Err(format!("rejected: {}", reason).into())
	}

	async fn lookup_role<'b>(&self, guild: &'b Guild, role: String) -> Option<&'b Role> {
		let alias = self.with_cfg(|cfg| cfg.resolve_alias(&role)).await;

		alias
			.and_then(|a| guild.roles.get(&RoleId(a)))
			.or_else(|| guild.role_by_name(&role))
	}

	async fn guild_or_reject<'b>(&self) -> CommandResult<Guild> {
		let guild = match self.msg.guild(&self.ctx).await {
			Some(g) => g,
			_ => return self.send_reject_with_msg("Not in a guild?").await,
		};
		Ok(guild)
	}

	async fn role_or_reject<'b>(&self, guild: &'b Guild, role: String) -> CommandResult<&'b Role> {
		let role = match self.lookup_role(&guild, role).await {
			Some(r) => r,
			_ => return self.send_reject_with_msg("Role not found").await,
		};
		Ok(role)
	}

	async fn assert_role_allowed(&self, role: &Role) -> CommandResult<()> {
		if !self
			.with_cfg(|cfg| cfg.get_roles().contains(&role.id.as_u64()))
			.await
		{
			return self.send_reject_with_msg("Not an allowed role").await;
		}
		Ok(())
	}
}

impl<'a> Api<'a> {
	#[instrument(level = "debug", skip(self), fields(author = %self.msg.author.tag()))]
	pub async fn list_roles(&self) -> CommandResult {
		debug!("listing roles");
		let guild = self.guild_or_reject().await?;

		let (roles, aliases) = self
			.with_cfg(|cfg| (cfg.get_roles().clone(), cfg.get_aliases().clone()))
			.await;

		let mut role_text = Vec::new();
		for role_id in roles {
			let aliases = aliases
				.iter()
				.filter(|(_, r)| **r == role_id)
				.map(|(a, _)| format!("`{}`", a))
				.collect::<Vec<_>>()
				.join(",");

			if let Some(text) = {
				guild.roles.get(&RoleId(role_id)).map(|r| {
					let main = format!("`{}`", r.name.clone());
					if aliases.len() > 0 {
						format!("{} - aliases: {}", main, aliases)
					} else {
						main
					}
				})
			} {
				role_text.push(text);
			}
		}

		self.msg
			.channel_id
			.send_message(&self.ctx, |m| {
				m.embed(|e| {
					e.title("Available Roles");
					e.description(role_text.join("\n"));
					e
				})
			})
			.await?;

		Ok(())
	}

	#[instrument(level = "debug", skip(self), fields(author = %self.msg.author.tag()))]
	pub async fn add_role(&self, role: String) -> CommandResult {
		debug!("adding role");
		let guild = self.guild_or_reject().await?;
		let role = self.role_or_reject(&guild, role).await?;

		self.assert_role_allowed(role).await?;

		let mut member = self.msg.member(&self.ctx).await?;

		if let Some(_) = member
			.roles(&self.ctx)
			.await
			.expect("Users always have roles")
			.iter()
			.find(|p| p.id == role.id)
		{
			return self
				.send_reject_with_msg("You already have this role")
				.await;
		}

		match member.add_role(&self.ctx, role.id).await {
			Ok(_) => Ok(()),
			_ => return self.send_reject_with_msg("Couldn't set role").await,
		}
	}

	#[instrument(level = "debug", skip(self), fields(author = %self.msg.author.tag()))]
	pub async fn remove_role(&self, role: String) -> CommandResult {
		debug!("removing role");
		let guild = self.guild_or_reject().await?;
		let role = self.role_or_reject(&guild, role).await?;

		self.assert_role_allowed(role).await?;

		let mut member = guild.member(&self.ctx, self.msg.author.id).await?;

		if let None = member
			.roles(&self.ctx)
			.await
			.expect("Users always have roles")
			.iter()
			.find(|p| p.id == role.id)
		{
			return self.send_reject_with_msg("You don't have this role").await;
		}

		match member.remove_role(&self.ctx, role.id).await {
			Ok(_) => Ok(()),
			_ => return self.send_reject_with_msg("Couldn't remove role").await,
		}
	}

	#[instrument(level = "debug", skip(self), fields(author = %self.msg.author.tag()))]
	pub async fn create_role(&self, role: String) -> CommandResult {
		debug!("create role");
		let guild = self.guild_or_reject().await?;

		if let Some(_) = guild.role_by_name(&role) {
			return self
				.send_reject_with_msg("A role with that name already exists")
				.await;
		}

		let role = guild
			.create_role(&self.ctx, |r| {
				r.name(role);
				r
			})
			.await?;

		self.with_cfg_mut(|cfg| {
			cfg.add_role(*role.id.as_u64());
		})
		.await;

		Ok(())
	}

	#[instrument(level = "debug", skip(self), fields(author = %self.msg.author.tag()))]
	pub async fn alias_role(&self, role: String, alias: String) -> CommandResult {
		debug!("alias role");
		let guild = self.guild_or_reject().await?;
		let role = self.role_or_reject(&guild, role).await?;

		if !self
			.with_cfg(|cfg| cfg.get_roles().contains(&role.id.as_u64()))
			.await
		{
			return self
				.send_reject_with_msg(format!(
					"'{}' is not an allowed role (aliases are only possible for allowed roles)",
					role.name
				))
				.await;
		}

		self.with_cfg_mut(move |cfg| {
			cfg.add_alias(*role.id.as_u64(), alias);
		})
		.await;

		Ok(())
	}

	#[instrument(level = "debug", skip(self), fields(author = %self.msg.author.tag()))]
	pub async fn remove_alias(&self, alias: String) -> CommandResult {
		debug!("remove alias");
		if !self
			.with_cfg(|cfg| cfg.get_aliases().contains_key(&alias))
			.await
		{
			return self
				.send_reject_with_msg(format!("There's no alias called '{}'", alias))
				.await;
		}

		self.with_cfg_mut(move |cfg| {
			cfg.remove_alias(alias);
		})
		.await;

		Ok(())
	}

	#[instrument(level = "debug", skip(self), fields(author = %self.msg.author.tag()))]
	pub async fn allow_role(&self, role: String) -> CommandResult {
		debug!("allow role");
		let guild = self.guild_or_reject().await?;
		let role = self.role_or_reject(&guild, role).await?;

		self.with_cfg_mut(|cfg| {
			cfg.add_role(*role.id.as_u64());
		})
		.await;

		Ok(())
	}

	#[instrument(level = "debug", skip(self), fields(author = %self.msg.author.tag()))]
	pub async fn deny_role(&self, role: String) -> CommandResult {
		debug!("deny role");
		let guild = self.guild_or_reject().await?;
		let role = self.role_or_reject(&guild, role).await?;

		self.assert_role_allowed(role).await?;

		self.with_cfg_mut(|cfg| {
			cfg.remove_role(*role.id.as_u64());
		})
		.await;

		Ok(())
	}
}
