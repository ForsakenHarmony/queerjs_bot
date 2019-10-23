use serenity::prelude::*;
use serenity::model::prelude::*;
use serenity::framework::standard::{CommandResult, CommandError};
use crate::store::{Config, ConfigKey};
use lock_api::{RwLockReadGuard, RwLockWriteGuard};
use parking_lot::RawRwLock;

type DiscordResult<T> = ::std::result::Result<T, CommandError>;
type ReadGuard<'a, T> = RwLockReadGuard<'a, RawRwLock, T>;
type WriteGuard<'a, T> = RwLockWriteGuard<'a, RawRwLock, T>;

pub struct Api<'a> {
  ctx: &'a mut Context,
	msg: &'a Message,
}

impl<'a> Api<'a> {
	pub fn new(ctx: &'a mut Context, msg: &'a Message) -> Self {
		Api {
			ctx,
			msg
		}
	}
}

impl<'a> Api<'a> {
	fn get_cfg<'b>(&self) -> ReadGuard<'b, Config> {
		let data = self.ctx.data.write();
		let cfg = data.get::<ConfigKey>().expect("Should have Config").clone();
		cfg.read()
	}

	fn get_cfg_mut<'b>(&self) -> WriteGuard<Config> {
		let data = self.ctx.data.write();
		let cfg = data.get::<ConfigKey>().expect("Should have Config").clone();
		cfg.write()
	}

	fn send_ok(&self) -> CommandResult {
		self.msg.react(&self.ctx, ReactionType::Unicode("✅".to_string()))?;
		Ok(())
	}

	fn send_reject(&self) -> CommandResult {
		self.msg.react(&self.ctx, ReactionType::Unicode("❌".to_string()))?;
		Err("#reject#".into())
	}

	fn send_reject_with_msg<T>(&self, reason: impl AsRef<str>) -> DiscordResult<T> {
		self.msg.react(&self.ctx, ReactionType::Unicode("❌".to_string()))?;
		let response = self.msg.reply(&self.ctx, reason)?;
		Err(format!("#reject#-#{}", response.id.as_u64()).into())
	}

	fn lookup_role<'b>(&self, guild: &'b Guild, role: String) -> Option<&'b Role> {
		let cfg = self.get_cfg();
		let alias = cfg.resolve_alias(&role);

		alias
			.and_then(|a| guild.roles.get(&RoleId(a)))
			.or_else(|| guild.role_by_name(&role))
	}

	fn guild_or_reject<'b>(&self, reason: impl AsRef<str>) -> DiscordResult<ReadGuard<Guild>> {
		let guild = match self.msg.guild(&self.ctx) {
			Some(g) => g,
			_ => return self.send_reject_with_msg(reason),
		};
		let guild = guild.read();
		Ok(guild)
	}
}

impl<'a> Api<'a> {
	pub fn list_roles(&self) -> CommandResult {
		let guild = self.guild_or_reject("Not in a guild?")?;

		let cfg = self.get_cfg();

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

		self.msg.channel_id.send_message(&self.ctx.http, |m| {
			m.embed(|e| {
				e.title("Available Roles");
				e.description(roles.join("\n"));
				e
			});
			m
		})?;

		Ok(())
	}

	pub fn add_role(&self, role: String) -> CommandResult {
		let guild = self.guild_or_reject("Not in a guild?")?;

		let role = match self.lookup_role(&guild, role) {
			Some(r) => r,
			_ => return self.send_reject_with_msg("Role not found"),
		};

		let cfg = self.get_cfg();
		let roles = cfg.get_roles();
		if !roles.contains(&role.id.as_u64()) { return self.send_reject_with_msg("Not an allowed role"); }

		let mut member = self.msg.member(&self.ctx).expect("We already checked this");

		if let Some(_) = member.roles(&self.ctx).expect("Users always have roles").iter().find(|p| p.id == role.id) {
			return self.send_reject_with_msg("You already have this role");
		}

		match member.add_role(&self.ctx, role.id) {
			Ok(_) => Ok(()),
			_ => return self.send_reject_with_msg("Couldn't set role")
		}
	}

	pub fn remove_role(&self, role: String) -> CommandResult {
		let guild = self.guild_or_reject("Not in a guild?")?;

		let role = match self.lookup_role(&guild, role) {
			Some(r) => r,
			_ => return self.send_reject_with_msg("Role not found"),
		};

		let cfg = self.get_cfg();
//		let cfg = cfg_guard.read();
		let roles = cfg.get_roles();
		if !roles.contains(&role.id.as_u64()) { return self.send_reject_with_msg("Not an allowed role"); }

		let mut member = self.msg.member(&self.ctx).expect("We already checked this");

		if let None = member.roles(&self.ctx).expect("Users always have roles").iter().find(|p| p.id == role.id) {
			return self.send_reject_with_msg("You don't have this role");
		}

		match member.remove_role(&self.ctx, role.id) {
			Ok(_) => Ok(()),
			_ => return self.send_reject_with_msg("Couldn't remove role")
		}
	}

	pub fn create_role(&self, role: String) -> CommandResult {
		let guild = self.guild_or_reject("Not in a guild?")?;

		if let Some(_) = guild.role_by_name(&role) {
			return self.send_reject_with_msg("A role with that name already exists");
		}

		let role = guild.create_role(&self.ctx, |r| {
			r.name(role);
			r
		})?;

		let mut cfg = self.get_cfg_mut();
		cfg.add_role(*role.id.as_u64());

		Ok(())
	}

	pub fn alias_role(&self, role: String, alias: String) -> CommandResult {
		let guild = self.guild_or_reject("Not in a guild?")?;

		let role = match guild.role_by_name(&role) {
			Some(r) => r,
			_ => return self.send_reject_with_msg("Role not found"),
		};

		{
			let cfg = self.get_cfg();
			let roles = cfg.get_roles();
			if !roles.contains(&role.id.as_u64()) { return self.send_reject_with_msg("Not an allowed role (aliases are only possible for allowed roles)"); }
		}

		{
			println!("wew");

			let mut cfg = self.get_cfg_mut();
			println!("myb?");
			cfg.add_alias(*role.id.as_u64(), alias);

			println!("no?");
		}

		Ok(())
	}

	pub fn allow_role(&self, role: String) -> CommandResult {
		let guild = self.guild_or_reject("Not in a guild?")?;

		let role = match guild.role_by_name(&role) {
			Some(r) => r,
			_ => return self.send_reject_with_msg("Role not found"),
		};

		let mut cfg = self.get_cfg_mut();
		cfg.add_role(*role.id.as_u64());

		Ok(())
	}

	pub fn deny_role(&self, role: String) -> CommandResult {
		let guild = self.guild_or_reject("Not in a guild?")?;

		let role = match guild.role_by_name(&role) {
			Some(r) => r,
			_ => return self.send_reject_with_msg("Role not found"),
		};

		{
			let cfg = self.get_cfg();
			let roles = cfg.get_roles();
			if !roles.contains(&role.id.as_u64()) { return self.send_reject_with_msg("Not an allowed role"); }
		}

		{
			let mut cfg = self.get_cfg_mut();
			cfg.remove_role(*role.id.as_u64());
		}

		Ok(())
	}
}