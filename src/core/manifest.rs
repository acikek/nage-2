use std::collections::{HashMap, HashSet};

use anyhow::{Result, anyhow};
use semver::Version;
use serde::Deserialize;

use crate::loading::base::Loader;

use super::{choice::{Variables, Notes, SoundAction, SoundActionMode}, text::{TextSpeed, TextLines, TemplatableValue}, player::PathEntry, resources::UnlockedInfoPages};

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
/// A collection of settings that identify information about the game itself and its authors.
pub struct Metadata {
	pub name: String,
	id: Option<String>,
	pub authors: Vec<String>,
	pub version: Version,
	pub contact: Option<Vec<String>>
}

impl Metadata {
	pub fn game_id(&self) -> &str {
		self.id.as_ref().unwrap_or(&self.name)
	}
}

#[derive(Deserialize, Debug)]
#[serde(default, deny_unknown_fields)]
pub struct HistorySettings {
	pub locked: bool,
	pub size: usize
}

impl Default for HistorySettings {
	fn default() -> Self {
		Self { 
			locked: false,
			size: 5
		}
	}
}

#[derive(Deserialize, Debug)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
	pub save: bool,
	pub debug: bool,
	pub speed: TextSpeed,
	pub history: HistorySettings,
	pub lang: Option<String>,
	pub channels: Option<HashMap<String, bool>>
}

impl Default for Settings {
	fn default() -> Self {
		Self {
			save: true,
			debug: false,
			speed: TextSpeed::Delay(TemplatableValue::value(5)),
			history: HistorySettings::default(),
			lang: None,
			channels: None
		}
	}
}

impl Settings {
	pub fn enabled_channels(&self) -> HashSet<String> {
		self.channels.as_ref().map(|map| {
			map.iter()
    			.filter(|(_, &enabled)| enabled)
				.map(|(key, _)| key.clone())
				.collect()
		})
		.unwrap_or(HashSet::new())
	}
}

#[derive(Deserialize, Debug, Clone)]
pub struct EntrypointSoundAction {
	name: String,
	channel: String,
	seek: Option<u64>,
	speed: Option<f64>
}

impl Into<SoundAction> for EntrypointSoundAction {
    fn into(self) -> SoundAction {
        SoundAction { 
			name: Some(self.name.into()), 
			channel: self.channel.into(), 
			mode: TemplatableValue::value(SoundActionMode::default()), 
			seek: self.seek.map(TemplatableValue::value), 
			speed: self.speed.map(TemplatableValue::value) 
		}
    }
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Entrypoint {
	pub path: PathEntry,
	pub background: Option<TextLines>,
	pub notes: Option<Notes>,
	pub variables: Option<Variables>,
	#[serde(rename = "info")]
	pub info_pages: Option<UnlockedInfoPages>,
	pub log: Option<Vec<String>>,
	pub sounds: Option<Vec<EntrypointSoundAction>>
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
	pub metadata: Metadata,
	pub settings: Settings,
	pub entry: Entrypoint
}

impl Manifest {
	pub const FILE: &'static str = "nage.yml";

	pub fn load(loader: &Loader) -> Result<Self> {
		let config: Self = loader.load_file(Self::FILE)?;
		config.validate()?;
		Ok(config)
	}

	fn validate(&self) -> Result<()> {
		let size = self.settings.history.size;
		if size == 0 {
			return Err(anyhow!("Failed to validate manifest: `settings.history.size` must be non-zero"));
		}
		Ok(())
	}
}