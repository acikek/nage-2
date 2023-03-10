use anyhow::Result;
use result::OptionResultExt;

use crate::{core::{player::Player, manifest::Manifest, choice::Choice, prompt::PromptModel, resources::Resources}, cmd::runtime::{RuntimeCommand, CommandResult}, game::input::{InputContext, InputResult}, loading::saves::SaveManager, text::{display::Text, context::TextContext}};

use super::input::InputController;

pub enum GameLoopResult {
	Retry(bool),
	Continue,
	Shutdown(bool)
}

pub fn handle_quit(shutdown: bool) -> GameLoopResult {
	use GameLoopResult::*;
	if shutdown { 
		Shutdown(false)
	}
	else { 
		println!("Signal quit again or use '.quit' to exit");
		Retry(true)
	}
}

pub fn handle_choice(choice: &Choice, config: &Manifest, player: &mut Player, resources: &Resources, model: &PromptModel, text_context: &TextContext) -> Result<GameLoopResult> {
	use GameLoopResult::*;
	player.choose_full(choice, None, config, resources, model, text_context)?;
	if let Some(ending) = &choice.ending {
		println!();
		Text::print_lines(ending, text_context)?;
		return Ok(Shutdown(true));
	}
	Ok(Continue)
}

pub fn handle_command(parse: Result<RuntimeCommand>, config: &Manifest, player: &mut Player, saves: &SaveManager, resources: &Resources, text_context: &TextContext) -> Result<GameLoopResult> {
	match &parse {
		Err(err) => println!("\n{err}"), // Clap error
		Ok(command) => {
			match command.run(config, player, saves, resources, text_context) {
				Err(err) => println!("Error: {err}"), // Command runtime error
				Ok(result) => {
					match result {
						CommandResult::Submit(loop_result) => return Ok(loop_result),
						CommandResult::Output(output) => println!("{output}")
					}
				}
			}
		}
	};
	Ok(GameLoopResult::Retry(parse.is_ok()))
}

pub fn take_input(input: &mut InputController, context: &InputContext, config: &Manifest, player: &mut Player, saves: &SaveManager, resources: &Resources, model: &PromptModel, text_context: &TextContext, choices: &Vec<&Choice>) -> Result<GameLoopResult> {
	use GameLoopResult::*;
	let result = match input.take(context) {
		Err(err) => {
			println!("{err}");
			Retry(true)
		},
		Ok(result) => match result {
			InputResult::Quit(shutdown) => handle_quit(shutdown),
			InputResult::Choice(i) => handle_choice(choices[i - 1], config, player, resources, model, text_context)?,
			InputResult::Variable(result) => {
				// Modify variables after the choose call since history entries are sensitive to this order
				player.choose(choices[0], Some(&result), config, model, resources, text_context)?;
				player.variables.insert(result.0.clone(), result.1.clone());
				player.try_push_log(choices[0], config, resources)?;
				Continue
			},
			InputResult::Command(parse) => handle_command(parse, config, player, saves, resources, text_context)?
		}
	};
	Ok(result)
}

pub fn next_input_context(model: &PromptModel, choices: &Vec<&Choice>, text_context: &TextContext) -> Result<Option<InputContext>> {
	use PromptModel::*;
	let result = match &model {
		Response => Some(InputContext::Choices(choices.len())),
		&Input(name, prompt) => Some(InputContext::Variable(name.clone(), prompt.map(|s| s.fill(text_context)).invert()?)),
		_ => None
	};
	Ok(result)
}