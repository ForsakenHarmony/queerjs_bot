use std::collections::HashMap;
use std::path::{PathBuf, Path};
use serenity::prelude::{TypeMapKey, RwLock};
use std::sync::Arc;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use serde_json::{from_reader, to_writer_pretty};
use serde_derive::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct InnerConfig {
	allowed_roles: Vec<u64>,
	aliases: HashMap<String, u64>,
}

pub struct Config {
	inner_config: InnerConfig,
	path: PathBuf,
}

pub struct ConfigKey;

impl TypeMapKey for ConfigKey {
	type Value = Arc<RwLock<Config>>;
}

impl Config {
	pub(crate) fn get_roles(&self) -> &Vec<u64> {
		&self.inner_config.allowed_roles
	}

	pub(crate) fn add_role(&mut self, role: u64) {
		if self.inner_config.allowed_roles.contains(&role) { return; }

		self.inner_config.allowed_roles.push(role);

		self.save();
	}

	pub(crate) fn remove_role(&mut self, role: u64) {
		if !self.inner_config.allowed_roles.contains(&role) { return; }

		self.inner_config.allowed_roles.remove(self.inner_config.allowed_roles.iter().position(|r| &role == r).unwrap());
		let to_remove = self.inner_config.aliases.iter().filter_map(|(a, r)| if *r == role { Some(a.clone()) } else { None }).collect::<Vec<_>>();
		to_remove.iter().for_each(|a| {
			self.inner_config.aliases.remove(a);
		});

		self.save();
	}

	pub(crate) fn add_alias(&mut self, role: u64, alias: String) {
		if !self.inner_config.allowed_roles.contains(&role) { return; }

		self.inner_config.aliases.insert(alias, role);

		self.save();
	}

	fn remove_alias(&mut self, alias: String) {
		self.inner_config.aliases.remove(&alias);

		self.save();
	}

	pub(crate) fn get_aliases(&self) -> &HashMap<String, u64> {
		&self.inner_config.aliases
	}

	pub(crate) fn resolve_alias(&self, alias: &String) -> Option<u64> {
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

	pub(crate) fn load_or_create<P: AsRef<Path>>(path: P) -> Config {
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
