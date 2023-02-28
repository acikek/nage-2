use std::collections::HashMap;

use anyhow::{Result, anyhow};

use crate::{input::controller::InputController, loading::{load_content, load_files}, core::{prompt::{Prompts, Prompt, PromptModel}, text::{Translations, Text}, manifest::Manifest, player::Player}};

use super::gloop::{next_input_context, take_input, GameLoopResult};

pub type Scripts = HashMap<String, String>;

#[derive(Debug)]
pub struct Resources {
	pub prompts: Prompts,
	pub translations: Translations,
	pub scripts: Scripts
}

impl Resources {
	pub fn load() -> Result<Self> {
		let result = Resources {
			prompts: load_content("prompts")?,
			translations: load_content("lang")?,
			scripts: load_files("scripts")?
		};
		Ok(result)
	}

	pub fn validate(&self) -> Result<()> {
		let _ = Prompt::validate_all(&self.prompts)?;
		Ok(())
	}
}

pub fn first_play_init(config: &Manifest, player: &mut Player, translations: &Translations) {
	if let Some(background) = &config.entry.background {
		let lang_file = translations.get(&player.lang);
		Text::print_lines_nl(background, &player.variables, lang_file, config);
	}
	player.began = true;
}

pub fn begin(config: &Manifest, player: &mut Player, resources: &Resources, input: &mut InputController) -> Result<bool> {
	if !player.began {
		first_play_init(config, player, &resources.translations);
	}
	let silent = 'outer: loop {
		let lang_file = resources.translations.get(&player.lang);
		let entry = player.latest_entry()?;
		let next_prompt = Prompt::get_from_path(&resources.prompts, &entry.path)?;
		let model = next_prompt.model();
		let choices = next_prompt.usable_choices(&player.notes);

		if choices.is_empty() {
			return Err(anyhow!("No usable choices"))
		}
		
		next_prompt.print(&model, entry.display, &choices, &player.variables, config, lang_file);

		match model {
			PromptModel::Redirect(choice) => player.choose(choice, None, config)?,
			PromptModel::Ending(lines) => {
				Text::print_lines(lines, &player.variables, lang_file, config);
				break 'outer true
			},
			_ => loop {
				let context = next_input_context(&model, &choices, &player.variables, lang_file, config)
					.ok_or(anyhow!("Could not resolve input context"))?;
				// Borrow-checker coercion; only using necessary fields in static method
				match take_input(input, &context, config, player, resources, lang_file, &choices)? {
					GameLoopResult::Retry(flush) => if flush { println!() },
					GameLoopResult::Continue => { println!(); break },
					GameLoopResult::Shutdown(silent) => break 'outer silent
				}
			}
		}
	};
	Ok(silent)
}

pub fn shutdown(config: &Manifest, player: &Player, silent: bool) {
	if config.settings.save {
		player.save();
	}
	if !silent {
		println!("Exiting...");
	}
}

pub fn crash_context(config: &Manifest) -> String {
	let contact = config.metadata.contact.as_ref().map(|info| {
		let strings: Vec<String> = info.iter()
			.map(|value| format!("- {value}"))
			.collect();
		format!("\n\nContact the developers:\n{}", strings.join("\n"))
	});
	format!("The game has crashed; it's not your fault!{}", contact.unwrap_or(String::new()))
}